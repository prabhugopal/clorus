use crate::interface::InterfaceFunction;
use crate::manifest::Manifest;
use clorus_ffi_gen::{FfiGenerator, FunctionInfo};
use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
/// Automatic Rust FFI processing - Phase 2 Architecture
/// Generates wrapper crates in target/rust-ffi/ instead of polluting library source
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct RustFfiProcessor {
    pub libraries: Vec<ProcessedLibrary>,
}

#[derive(Debug, Clone)]
pub struct ProcessedLibrary {
    pub name: String,
    pub static_lib_path: PathBuf,
    pub dynamic_lib_path: Option<PathBuf>,
    pub functions: Vec<FunctionInfo>,
}

#[derive(Debug, Clone)]
enum RustDepSource {
    Path(PathBuf),
    Version(String),
}

#[derive(Debug, Deserialize)]
struct CargoMetadata {
    packages: Vec<CargoPackage>,
}

#[derive(Debug, Deserialize)]
struct CargoPackage {
    name: String,
    version: String,
    source: Option<String>,
    targets: Vec<CargoTarget>,
}

#[derive(Debug, Deserialize)]
struct CargoTarget {
    kind: Vec<String>,
    src_path: String,
}

#[derive(Debug, Clone)]
struct InterfaceBinding {
    exposed_name: String,
    rust_symbol: String,
    params: Vec<clorus_ffi_gen::ParamInfo>,
    return_type: String,
}

impl RustFfiProcessor {
    fn is_valid_export_symbol(name: &str) -> bool {
        let mut chars = name.chars();
        let Some(first) = chars.next() else {
            return false;
        };
        if !(first == '_' || first.is_ascii_alphabetic()) {
            return false;
        }
        chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
    }

    fn is_valid_rust_symbol_path(symbol: &str) -> bool {
        if symbol.is_empty() || symbol.contains(char::is_whitespace) {
            return false;
        }
        symbol.split("::").all(|segment| {
            let mut chars = segment.chars();
            let Some(first) = chars.next() else {
                return false;
            };
            if !(first == '_' || first.is_ascii_alphabetic()) {
                return false;
            }
            chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
        })
    }

    fn compare_version_like(a: &str, b: &str) -> std::cmp::Ordering {
        fn parse_parts(v: &str) -> Vec<u64> {
            v.split('.')
                .map(|part| {
                    part.chars()
                        .take_while(|c| c.is_ascii_digit())
                        .collect::<String>()
                        .parse::<u64>()
                        .unwrap_or(0)
                })
                .collect()
        }

        let av = parse_parts(a);
        let bv = parse_parts(b);
        let max_len = av.len().max(bv.len());

        for i in 0..max_len {
            let ai = *av.get(i).unwrap_or(&0);
            let bi = *bv.get(i).unwrap_or(&0);
            match ai.cmp(&bi) {
                std::cmp::Ordering::Equal => {}
                non_eq => return non_eq,
            }
        }

        std::cmp::Ordering::Equal
    }

    fn should_build_offline(source: &RustDepSource) -> bool {
        matches!(source, RustDepSource::Path(_))
    }

    fn is_supported_ffi_type(type_name: &str) -> bool {
        matches!(
            type_name,
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
                | "isize"
                | "usize"
                | "bool"
                | "String"
                | "()"
                | "*mut u8"
        )
    }

    fn unsupported_signature_reason(function: &FunctionInfo) -> Option<String> {
        for param in &function.params {
            if !Self::is_supported_ffi_type(&param.type_name) {
                return Some(format!(
                    "{}: unsupported param `{}` type `{}`",
                    function.name, param.name, param.type_name
                ));
            }
        }

        if !Self::is_supported_ffi_type(&function.return_type) {
            return Some(format!(
                "{}: unsupported return type `{}`",
                function.name, function.return_type
            ));
        }

        None
    }

    fn collect_unsupported_signature_details(functions: &[FunctionInfo]) -> Vec<String> {
        functions
            .iter()
            .filter_map(Self::unsupported_signature_reason)
            .collect()
    }

    fn likely_impl_method_only_api(src_path: &Path) -> bool {
        let Ok(source) = fs::read_to_string(src_path) else {
            return false;
        };

        // Heuristic: impl blocks with pub methods are present.
        // Auto-discovery currently focuses on top-level pub fn items.
        source.contains("impl ") && source.contains("pub fn ")
    }

    fn resolve_interface_path(
        dep_name: &str,
        interface_spec: crate::manifest::InterfaceSpec,
    ) -> Result<String, String> {
        match interface_spec {
            crate::manifest::InterfaceSpec::Path(path) => {
                let trimmed = path.trim();
                if trimmed.is_empty() {
                    return Err(format!(
                        "Interface path for '{}' cannot be empty. Use a valid .clri or .clorus-ffi file path.",
                        dep_name
                    ));
                }
                let path_obj = std::path::Path::new(trimmed);
                let is_supported_ext = matches!(
                    path_obj.extension().and_then(|s| s.to_str()),
                    Some("clri") | Some("clorus-ffi")
                );
                if !is_supported_ext {
                    return Err(format!(
                        "Unsupported interface file extension for '{}': '{}'. Expected a .clri or .clorus-ffi file.",
                        dep_name, trimmed
                    ));
                }
                Ok(trimmed.to_string())
            }
            crate::manifest::InterfaceSpec::Auto => {
                let clri_path = format!("interfaces/{}.clri", dep_name);
                let legacy_path = format!("interfaces/{}.clorus-ffi", dep_name);

                if std::path::Path::new(&clri_path).exists() {
                    return Ok(clri_path);
                }
                if std::path::Path::new(&legacy_path).exists() {
                    return Ok(legacy_path);
                }

                Err(format!(
                    "Interface auto-discovery failed for '{}'. Searched:\n  - {}\n  - {}\nAdd one of these files, or set interface = \"<path>\" explicitly.",
                    dep_name, clri_path, legacy_path
                ))
            }
        }
    }

    fn retain_supported_ffi_functions(
        functions: Vec<FunctionInfo>,
        verbose: bool,
    ) -> Vec<FunctionInfo> {
        let unsupported_details = Self::collect_unsupported_signature_details(&functions);
        let filtered: Vec<FunctionInfo> = functions
            .into_iter()
            .filter(|f| {
                f.params
                    .iter()
                    .all(|p| Self::is_supported_ffi_type(&p.type_name))
                    && Self::is_supported_ffi_type(&f.return_type)
            })
            .collect();

        if verbose {
            let skipped = unsupported_details.len();
            if skipped > 0 {
                println!(
                    "      Skipped {} unsupported function(s) (non-FFI-compatible signature)",
                    skipped
                );
                for detail in unsupported_details.iter().take(3) {
                    println!("         - {}", detail);
                }
            }
        }

        filtered
    }

    pub fn new() -> Self {
        Self {
            libraries: Vec::new(),
        }
    }

    /// Process all Rust dependencies from manifest
    pub fn process_dependencies(manifest: &Manifest, verbose: bool) -> Result<Self, String> {
        let mut processor = Self::new();

        if manifest.rust_dependencies.is_empty() {
            return Ok(processor);
        }

        if verbose {
            println!(
                "   Processing {} Rust dependencies...",
                manifest.rust_dependencies.len()
            );
        }

        for (name, dep) in &manifest.rust_dependencies {
            if verbose {
                println!("   → {}", name);
            }

            let source = if let Some(path) = dep.get_path() {
                let lib_path = PathBuf::from(path);
                if !lib_path.exists() {
                    return Err(format!(
                        "Rust dependency '{}' path not found: {}",
                        name,
                        lib_path.display()
                    ));
                }
                RustDepSource::Path(lib_path)
            } else if let Some(version) = dep.get_version() {
                RustDepSource::Version(version.to_string())
            } else {
                return Err(format!(
                    "Rust dependency '{}' must specify either path or version",
                    name
                ));
            };

            // Check if interface file is specified
            let processed = if let Some(interface_spec) = dep.get_interface() {
                // Phase 2a: Use interface file
                let interface_path = Self::resolve_interface_path(name, interface_spec)?;

                if verbose {
                    println!("      Using interface file: {}", interface_path);
                }
                Self::create_wrapper_from_interface(name, &source, &interface_path, verbose)?
            } else {
                match &source {
                    // Phase 1: Auto-parse from source path
                    RustDepSource::Path(lib_path) => {
                        if verbose {
                            println!("      Auto-parsing from source");
                        }
                        Self::create_and_compile_wrapper(name, lib_path, verbose)?
                    }
                    RustDepSource::Version(version) => {
                        if verbose {
                            println!("      Auto-discovering from registry source");
                        }
                        Self::create_and_compile_wrapper_from_registry(name, version, verbose)?
                    }
                }
            };

            processor.libraries.push(processed);
        }

        Ok(processor)
    }

    /// Create wrapper crate in target/rust-ffi/ and compile it
    fn create_and_compile_wrapper(
        name: &str,
        lib_path: &Path,
        verbose: bool,
    ) -> Result<ProcessedLibrary, String> {
        // Create target/rust-ffi directory
        let rust_ffi_dir = PathBuf::from("target/rust-ffi");
        fs::create_dir_all(&rust_ffi_dir)
            .map_err(|e| format!("Failed to create target/rust-ffi: {}", e))?;

        // Wrapper crate name
        let wrapper_name = format!("{}_ffi", name.replace('-', "_"));
        let wrapper_dir = rust_ffi_dir.join(&wrapper_name);

        if verbose {
            println!("      Creating wrapper crate: {}", wrapper_dir.display());
        }

        // Create wrapper crate structure
        fs::create_dir_all(wrapper_dir.join("src"))
            .map_err(|e| format!("Failed to create wrapper crate directory: {}", e))?;

        // Parse the original library to get function signatures
        let lib_src = lib_path.join("src/lib.rs");
        if !lib_src.exists() {
            return Err(format!(
                "Rust library source not found: {}",
                lib_src.display()
            ));
        }

        let mut generator = FfiGenerator::new();
        generator.parse_file(&lib_src)?;
        generator.functions = Self::retain_supported_ffi_functions(generator.functions, verbose);

        if generator.functions.is_empty() {
            if verbose {
                println!("      No public functions found - creating empty wrapper");
                if Self::likely_impl_method_only_api(&lib_src) {
                    println!(
                        "      Hint: impl methods are not auto-discovered. Expose free bridge functions or use an interface + bridge API."
                    );
                }
            }
        } else if verbose {
            println!("      Found {} public functions", generator.functions.len());
        }

        // Generate Cargo.toml for wrapper crate
        Self::generate_wrapper_cargo_toml(&wrapper_dir, &wrapper_name, name, lib_path)?;

        // Generate lib.rs with FFI wrappers
        Self::generate_wrapper_lib_rs(&wrapper_dir, name, &generator)?;

        if verbose {
            println!("      Generated wrapper crate: {}", wrapper_dir.display());
        }

        // Compile the wrapper crate and return with function info
        let mut processed =
            Self::compile_wrapper_crate(&wrapper_dir, &wrapper_name, verbose, true)?;
        processed.functions = generator.functions.clone();

        if verbose {
            println!(
                "      Populated with {} function metadata entries",
                processed.functions.len()
            );
        }

        Ok(processed)
    }

    fn create_and_compile_wrapper_from_registry(
        name: &str,
        version_req: &str,
        verbose: bool,
    ) -> Result<ProcessedLibrary, String> {
        let rust_ffi_dir = PathBuf::from("target/rust-ffi");
        fs::create_dir_all(&rust_ffi_dir)
            .map_err(|e| format!("Failed to create target/rust-ffi: {}", e))?;

        let wrapper_name = format!("{}_ffi", name.replace('-', "_"));
        let wrapper_dir = rust_ffi_dir.join(&wrapper_name);

        if verbose {
            println!("      Creating wrapper crate: {}", wrapper_dir.display());
        }

        fs::create_dir_all(wrapper_dir.join("src"))
            .map_err(|e| format!("Failed to create wrapper crate directory: {}", e))?;

        // Ensure metadata command has a valid source tree.
        fs::write(wrapper_dir.join("src/lib.rs"), "// metadata probe\n")
            .map_err(|e| format!("Failed to write metadata probe source: {}", e))?;

        Self::generate_wrapper_cargo_toml_for_source(
            &wrapper_dir,
            &wrapper_name,
            name,
            &RustDepSource::Version(version_req.to_string()),
        )?;

        let discovered = Self::discover_registry_functions(&wrapper_dir, name, verbose)?;
        let unsupported_details = Self::collect_unsupported_signature_details(&discovered);
        let functions = Self::retain_supported_ffi_functions(discovered, verbose);
        if functions.is_empty() {
            let mut error = format!(
                "Rust dependency '{}' resolved from registry but no FFI-compatible functions were discovered in its lib target. Add an interface file (interfaces/{}.clri) or use a local bridge crate.",
                name, name
            );
            error.push_str(
                "\nCommon cause: API is primarily impl/associated methods; auto-discovery currently targets top-level pub fn.",
            );
            error.push_str(
                "\nRecommendation: provide bridge free functions and bind them via .clri interface.",
            );
            if !unsupported_details.is_empty() {
                error.push_str("\nUnsupported signature examples:");
                for detail in unsupported_details.iter().take(5) {
                    error.push_str(&format!("\n  - {}", detail));
                }
            }
            return Err(error);
        }

        if verbose {
            println!("      Discovered {} public function(s)", functions.len());
        }

        let mut generator = FfiGenerator::new();
        generator.functions = functions.clone();
        Self::generate_wrapper_lib_rs(&wrapper_dir, name, &generator)?;

        let mut processed =
            Self::compile_wrapper_crate(&wrapper_dir, &wrapper_name, verbose, false)?;
        processed.functions = functions;
        Ok(processed)
    }

    fn discover_registry_functions(
        wrapper_dir: &Path,
        dep_name: &str,
        verbose: bool,
    ) -> Result<Vec<FunctionInfo>, String> {
        let output = Command::new("cargo")
            .arg("metadata")
            .arg("--format-version")
            .arg("1")
            .current_dir(wrapper_dir)
            .output()
            .map_err(|e| format!("Failed to run cargo metadata for '{}': {}", dep_name, e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "Failed to resolve registry dependency '{}' via cargo metadata:\n{}",
                dep_name, stderr
            ));
        }

        let metadata: CargoMetadata = serde_json::from_slice(&output.stdout)
            .map_err(|e| format!("Failed to parse cargo metadata JSON: {}", e))?;

        let (src_path, version) = Self::resolve_registry_lib_src(&metadata, dep_name)?;
        if !src_path.exists() {
            return Err(format!(
                "Resolved source path for '{}' does not exist: {}",
                dep_name,
                src_path.display()
            ));
        }

        if verbose {
            println!(
                "      Registry source: {} (v{})",
                src_path.display(),
                version
            );
        }

        let mut generator = FfiGenerator::new();
        generator.parse_file(&src_path)?;
        Ok(generator.functions)
    }

    fn resolve_registry_lib_src(
        metadata: &CargoMetadata,
        dep_name: &str,
    ) -> Result<(PathBuf, String), String> {
        let package = metadata
            .packages
            .iter()
            .filter(|p| {
                p.name == dep_name && p.source.as_deref().unwrap_or("").starts_with("registry+")
            })
            .max_by(|a, b| Self::compare_version_like(&a.version, &b.version))
            .ok_or_else(|| {
                format!(
                    "Registry dependency '{}' was not found in cargo metadata package set",
                    dep_name
                )
            })?;

        let lib_target = package
            .targets
            .iter()
            .find(|t| t.kind.iter().any(|k| k == "lib"))
            .ok_or_else(|| {
                format!(
                    "Registry dependency '{}' (resolved {}) has no lib target",
                    dep_name, package.version
                )
            })?;

        Ok((PathBuf::from(&lib_target.src_path), package.version.clone()))
    }

    /// Generate Cargo.toml for the wrapper crate
    fn generate_wrapper_cargo_toml(
        wrapper_dir: &Path,
        wrapper_name: &str,
        original_name: &str,
        original_path: &Path,
    ) -> Result<(), String> {
        // Convert original path to absolute
        let original_abs = original_path
            .canonicalize()
            .map_err(|e| format!("Failed to canonicalize library path: {}", e))?;

        let cargo_toml_content = format!(
            r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "staticlib", "rlib"]

[dependencies]
{} = {{ path = "{}" }}
"#,
            wrapper_name,
            original_name, // Keep package name with hyphens
            original_abs.display()
        );

        fs::write(wrapper_dir.join("Cargo.toml"), cargo_toml_content)
            .map_err(|e| format!("Failed to write wrapper Cargo.toml: {}", e))
    }

    fn generate_wrapper_cargo_toml_for_source(
        wrapper_dir: &Path,
        wrapper_name: &str,
        original_name: &str,
        source: &RustDepSource,
    ) -> Result<(), String> {
        let dependency_spec = match source {
            RustDepSource::Path(original_path) => {
                let original_abs = original_path
                    .canonicalize()
                    .map_err(|e| format!("Failed to canonicalize library path: {}", e))?;
                format!("{{ path = \"{}\" }}", original_abs.display())
            }
            RustDepSource::Version(version) => format!("\"{}\"", version),
        };

        let cargo_toml_content = format!(
            r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "staticlib", "rlib"]

[dependencies]
{} = {}
"#,
            wrapper_name, original_name, dependency_spec
        );

        fs::write(wrapper_dir.join("Cargo.toml"), cargo_toml_content)
            .map_err(|e| format!("Failed to write wrapper Cargo.toml: {}", e))
    }

    /// Generate lib.rs for the wrapper crate with FFI wrappers
    fn generate_wrapper_lib_rs(
        wrapper_dir: &Path,
        original_name: &str,
        generator: &FfiGenerator,
    ) -> Result<(), String> {
        let lib_name = original_name.replace('-', "_");

        let mut content = format!(
            "// Auto-generated FFI wrapper crate for {}\n\
             // This crate wraps the original library and provides C-compatible FFI functions\n\
             \n\
             use {}::*;\n\
             \n",
            original_name, lib_name
        );

        // Add generated FFI wrappers
        if !generator.functions.is_empty() {
            content.push_str(&generator.generate_c_wrappers());
        }

        fs::write(wrapper_dir.join("src/lib.rs"), content)
            .map_err(|e| format!("Failed to write wrapper lib.rs: {}", e))
    }

    fn rust_to_c_type(rust_type: &str) -> String {
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
            // Use fixed-width C ABI carrier types for pointer-sized integers.
            "isize" => "i64".to_string(),
            "usize" => "u64".to_string(),
            "bool" => "bool".to_string(),
            "String" => "*mut c_char".to_string(),
            "()" => "()".to_string(),
            "*mut u8" => "*mut u8".to_string(),
            _ => "*mut u8".to_string(),
        }
    }

    fn c_to_rust_conversion(name: &str, rust_type: &str) -> String {
        match rust_type {
            "f32" | "f64" | "i8" | "u8" | "i16" | "u16" | "i32" | "u32" | "i64" | "u64"
            | "bool" | "*mut u8" => {
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

    fn rust_to_c_conversion(name: &str, rust_type: &str) -> String {
        match rust_type {
            "f32" | "f64" | "i8" | "u8" | "i16" | "u16" | "i32" | "u32" | "i64" | "u64"
            | "bool" | "*mut u8" => {
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

    fn generate_wrapper_lib_rs_from_interface(
        wrapper_dir: &Path,
        original_name: &str,
        bindings: &[InterfaceBinding],
    ) -> Result<(), String> {
        let lib_name = original_name.replace('-', "_");
        let mut content = format!(
            "// Auto-generated interface-based FFI wrapper crate for {}\n\
             use {}::*;\n\
             use std::ffi::{{CStr, CString}};\n\
             use std::os::raw::c_char;\n\n",
            original_name, lib_name
        );

        for binding in bindings {
            let wrapper_name = format!("clorus_{}", binding.exposed_name);
            let c_params: Vec<String> = binding
                .params
                .iter()
                .map(|p| format!("{}: {}", p.name, Self::rust_to_c_type(&p.type_name)))
                .collect();
            let c_return = Self::rust_to_c_type(&binding.return_type);
            let param_conversions: Vec<String> = binding
                .params
                .iter()
                .map(|p| Self::c_to_rust_conversion(&p.name, &p.type_name))
                .collect();
            let call_params: Vec<String> = binding
                .params
                .iter()
                .map(|p| format!("{}_rust", p.name))
                .collect();
            let return_conversion = Self::rust_to_c_conversion("result", &binding.return_type);

            content.push_str(&format!(
                "#[no_mangle]\npub extern \"C\" fn {}({}) -> {} {{\n{}\n    let result = {}({});\n{}\n}}\n\n",
                wrapper_name,
                c_params.join(", "),
                c_return,
                param_conversions.join("\n"),
                binding.rust_symbol,
                call_params.join(", "),
                return_conversion
            ));
        }

        fs::write(wrapper_dir.join("src/lib.rs"), content)
            .map_err(|e| format!("Failed to write interface wrapper lib.rs: {}", e))
    }

    /// Compile the wrapper crate
    fn compile_wrapper_crate(
        wrapper_dir: &Path,
        wrapper_name: &str,
        verbose: bool,
        offline: bool,
    ) -> Result<ProcessedLibrary, String> {
        if verbose {
            if offline {
                println!("      Compiling wrapper crate (offline)...");
            } else {
                println!("      Compiling wrapper crate (network-enabled)...");
            }
        }

        // Path deps can be built offline; version deps may need registry access.
        let mut cmd = Command::new("cargo");
        cmd.arg("build").arg("--release");
        if offline {
            cmd.arg("--offline");
        }
        let output = cmd
            .current_dir(wrapper_dir)
            .output()
            .map_err(|e| format!("Failed to run cargo build on wrapper: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let pointer_hint = if stderr.contains("expected `*mut")
                && stderr.contains("found `*mut u8`")
            {
                "\nHint: auto-discovery supports raw pointers only as `*mut u8`. \
Use an explicit `.clri` interface with supported types or a local bridge crate."
            } else {
                ""
            };
            return Err(format!(
                "Wrapper crate build failed:\n{}{}",
                stderr, pointer_hint
            ));
        }

        // Find compiled libraries
        #[cfg(target_os = "macos")]
        let dylib_name = format!("lib{}.dylib", wrapper_name);

        #[cfg(target_os = "linux")]
        let dylib_name = format!("lib{}.so", wrapper_name);

        #[cfg(target_os = "windows")]
        let dylib_name = format!("{}.dll", wrapper_name);

        let static_name = format!("lib{}.a", wrapper_name);

        let static_lib_path = wrapper_dir.join("target/release").join(&static_name);
        let dynamic_lib_path = wrapper_dir.join("target/release").join(&dylib_name);

        if !static_lib_path.exists() {
            return Err(format!(
                "Wrapper static library not found: {}",
                static_lib_path.display()
            ));
        }

        if verbose {
            println!("      Built wrapper: {}", static_lib_path.display());
            if dynamic_lib_path.exists() {
                println!("      Built wrapper: {}", dynamic_lib_path.display());
            }
        }

        Ok(ProcessedLibrary {
            name: wrapper_name.to_string(),
            static_lib_path,
            dynamic_lib_path: if dynamic_lib_path.exists() {
                Some(dynamic_lib_path)
            } else {
                None
            },
            functions: Vec::new(), // Will be populated by caller
        })
    }

    /// Create wrapper crate from interface file (Phase 2a)
    fn create_wrapper_from_interface(
        name: &str,
        source: &RustDepSource,
        interface_path: &str,
        verbose: bool,
    ) -> Result<ProcessedLibrary, String> {
        // Parse interface file
        let interface_file_path = PathBuf::from(interface_path);
        if !interface_file_path.exists() {
            return Err(format!(
                "Interface file not found: {}",
                interface_file_path.display()
            ));
        }
        if interface_file_path.is_dir() {
            return Err(format!(
                "Interface path points to a directory, not a file: {}",
                interface_file_path.display()
            ));
        }

        use crate::interface::parse_interface_file;
        let interface = parse_interface_file(&interface_file_path)?;

        if verbose {
            println!(
                "      Parsed interface: {} functions",
                interface.functions.len()
            );
        }
        if interface.functions.is_empty() {
            return Err(format!(
                "Interface '{}' contains no function definitions. Add at least one `(fn ...)` entry.",
                interface_path
            ));
        }

        // Convert interface functions to wrapper bindings and exposed symbols.
        let bindings: Vec<InterfaceBinding> = interface
            .functions
            .iter()
            .map(|f: &InterfaceFunction| {
                let exposed_name = f.name.replace('-', "_");
                let rust_symbol = match f.rust_symbol.clone() {
                    Some(symbol) => {
                        let trimmed = symbol.trim();
                        if trimmed.is_empty() {
                            return Err(format!(
                                "Invalid :rust override for function '{}': symbol cannot be empty in {}",
                                f.name, interface_path
                            ));
                        }
                        if !Self::is_valid_rust_symbol_path(trimmed) {
                            return Err(format!(
                                "Invalid :rust override for function '{}': '{}' is not a valid Rust path symbol in {}",
                                f.name, trimmed, interface_path
                            ));
                        }
                        trimmed.to_string()
                    }
                    None => exposed_name.clone(),
                };
                if !Self::is_valid_export_symbol(&exposed_name) {
                    return Err(format!(
                        "Interface '{}' defines function '{}' which normalizes to invalid export symbol '{}'. Use only [A-Za-z0-9_-] and avoid leading digits.",
                        interface_path, f.name, exposed_name
                    ));
                }
                Ok(InterfaceBinding {
                    exposed_name,
                    rust_symbol,
                    params: f
                        .params
                        .iter()
                        .map(|p| clorus_ffi_gen::ParamInfo {
                            name: p.name.clone(),
                            type_name: p.type_name.clone(),
                        })
                        .collect(),
                    return_type: f.return_type.clone(),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        for binding in &bindings {
            for param in &binding.params {
                if !Self::is_supported_ffi_type(&param.type_name) {
                    return Err(format!(
                        "Interface '{}' function '{}' has unsupported param type '{}'. Supported types: f32,f64,i8,u8,i16,u16,i32,u32,i64,u64,isize,usize,bool,String,(),*mut u8",
                        interface_path, binding.exposed_name, param.type_name
                    ));
                }
            }
            if !Self::is_supported_ffi_type(&binding.return_type) {
                return Err(format!(
                    "Interface '{}' function '{}' has unsupported return type '{}'. Supported types: f32,f64,i8,u8,i16,u16,i32,u32,i64,u64,isize,usize,bool,String,(),*mut u8",
                    interface_path, binding.exposed_name, binding.return_type
                ));
            }
        }
        let mut exposed_names = HashSet::new();
        for binding in &bindings {
            if !exposed_names.insert(binding.exposed_name.clone()) {
                return Err(format!(
                    "Interface '{}' defines duplicate exported function name '{}' after normalization. Rename one function to avoid wrapper symbol collision.",
                    interface_path, binding.exposed_name
                ));
            }
        }

        let functions: Vec<FunctionInfo> = bindings
            .iter()
            .map(|b| FunctionInfo {
                name: b.exposed_name.clone(),
                params: b.params.clone(),
                return_type: b.return_type.clone(),
            })
            .collect();

        // Create wrapper crate (same as auto-parse, but with interface functions)
        let rust_ffi_dir = PathBuf::from("target/rust-ffi");
        fs::create_dir_all(&rust_ffi_dir)
            .map_err(|e| format!("Failed to create target/rust-ffi: {}", e))?;

        let wrapper_name = format!("{}_ffi", name.replace('-', "_"));
        let wrapper_dir = rust_ffi_dir.join(&wrapper_name);

        if verbose {
            println!("      Creating wrapper crate: {}", wrapper_dir.display());
        }

        fs::create_dir_all(wrapper_dir.join("src"))
            .map_err(|e| format!("Failed to create wrapper crate directory: {}", e))?;

        // Generate Cargo.toml
        Self::generate_wrapper_cargo_toml_for_source(&wrapper_dir, &wrapper_name, name, source)?;

        // Generate lib.rs using interface functions with optional rust symbol overrides.
        Self::generate_wrapper_lib_rs_from_interface(&wrapper_dir, name, &bindings)?;

        if verbose {
            println!("      Generated wrapper from interface");
        }

        // Compile and return
        let mut processed = Self::compile_wrapper_crate(
            &wrapper_dir,
            &wrapper_name,
            verbose,
            Self::should_build_offline(source),
        )
        .map_err(|e| {
            format!(
                "Failed while compiling wrapper generated from interface '{}': {}",
                interface_path, e
            )
        })?;
        processed.functions = functions;

        Ok(processed)
    }

    /// Get paths to all static libraries for linking
    pub fn get_static_lib_paths(&self) -> Vec<PathBuf> {
        self.libraries
            .iter()
            .map(|lib| lib.static_lib_path.clone())
            .collect()
    }

    /// Get paths to all dynamic libraries for JIT loading
    pub fn get_dynamic_lib_paths(&self) -> Vec<PathBuf> {
        self.libraries
            .iter()
            .filter_map(|lib| lib.dynamic_lib_path.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{catch_unwind, resume_unwind, UnwindSafe};
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn cwd_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn with_cwd<T, F>(dir: &Path, f: F) -> T
    where
        F: FnOnce() -> T + UnwindSafe,
    {
        let _guard = cwd_lock().lock().expect("cwd lock poisoned");
        let original = std::env::current_dir().expect("get cwd");
        std::env::set_current_dir(dir).expect("cd temp root");
        let result = catch_unwind(f);
        std::env::set_current_dir(original).expect("restore cwd");
        match result {
            Ok(value) => value,
            Err(payload) => resume_unwind(payload),
        }
    }

    #[test]
    fn generate_wrapper_cargo_toml_for_version_dependency() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let wrapper_dir = std::env::temp_dir().join(format!("clorus_rustffi_test_{}", unique));
        std::fs::create_dir_all(&wrapper_dir).expect("create temp wrapper dir");

        let result = RustFfiProcessor::generate_wrapper_cargo_toml_for_source(
            &wrapper_dir,
            "demo_math_ffi",
            "demo-math",
            &RustDepSource::Version("0.1.0".to_string()),
        );
        assert!(
            result.is_ok(),
            "failed to generate cargo toml: {:?}",
            result.err()
        );

        let cargo_toml =
            std::fs::read_to_string(wrapper_dir.join("Cargo.toml")).expect("read generated cargo");
        assert!(cargo_toml.contains("demo-math = \"0.1.0\""));

        let _ = std::fs::remove_dir_all(wrapper_dir);
    }

    #[test]
    fn path_sources_build_offline_registry_sources_build_online() {
        assert!(RustFfiProcessor::should_build_offline(
            &RustDepSource::Path(PathBuf::from("."))
        ));
        assert!(!RustFfiProcessor::should_build_offline(
            &RustDepSource::Version("1.0".to_string())
        ));
    }

    #[test]
    fn resolve_registry_lib_src_prefers_registry_package_with_lib_target() {
        let metadata = CargoMetadata {
            packages: vec![
                CargoPackage {
                    name: "demo-math".to_string(),
                    version: "0.1.0".to_string(),
                    source: Some("path+file:///tmp/demo-math".to_string()),
                    targets: vec![CargoTarget {
                        kind: vec!["lib".to_string()],
                        src_path: "/tmp/local/src/lib.rs".to_string(),
                    }],
                },
                CargoPackage {
                    name: "demo-math".to_string(),
                    version: "0.2.0".to_string(),
                    source: Some(
                        "registry+https://github.com/rust-lang/crates.io-index".to_string(),
                    ),
                    targets: vec![CargoTarget {
                        kind: vec!["lib".to_string()],
                        src_path: "/tmp/registry/src/lib.rs".to_string(),
                    }],
                },
            ],
        };

        let (src, version) =
            RustFfiProcessor::resolve_registry_lib_src(&metadata, "demo-math").expect("resolve");
        assert_eq!(version, "0.2.0");
        assert_eq!(src, PathBuf::from("/tmp/registry/src/lib.rs"));
    }

    #[test]
    fn resolve_registry_lib_src_prefers_highest_registry_version() {
        let metadata = CargoMetadata {
            packages: vec![
                CargoPackage {
                    name: "demo-math".to_string(),
                    version: "0.10.0".to_string(),
                    source: Some(
                        "registry+https://github.com/rust-lang/crates.io-index".to_string(),
                    ),
                    targets: vec![CargoTarget {
                        kind: vec!["lib".to_string()],
                        src_path: "/tmp/registry/v0_10/src/lib.rs".to_string(),
                    }],
                },
                CargoPackage {
                    name: "demo-math".to_string(),
                    version: "0.9.2".to_string(),
                    source: Some(
                        "registry+https://github.com/rust-lang/crates.io-index".to_string(),
                    ),
                    targets: vec![CargoTarget {
                        kind: vec!["lib".to_string()],
                        src_path: "/tmp/registry/v0_9/src/lib.rs".to_string(),
                    }],
                },
            ],
        };

        let (src, version) =
            RustFfiProcessor::resolve_registry_lib_src(&metadata, "demo-math").expect("resolve");
        assert_eq!(version, "0.10.0");
        assert_eq!(src, PathBuf::from("/tmp/registry/v0_10/src/lib.rs"));
    }

    #[test]
    fn compare_version_like_handles_patch_width() {
        assert_eq!(
            RustFfiProcessor::compare_version_like("1.2.10", "1.2.2"),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            RustFfiProcessor::compare_version_like("1.0", "1.0.0"),
            std::cmp::Ordering::Equal
        );
    }

    #[test]
    fn is_valid_export_symbol_accepts_c_ffi_safe_identifiers() {
        assert!(RustFfiProcessor::is_valid_export_symbol("foo"));
        assert!(RustFfiProcessor::is_valid_export_symbol("_foo_2"));
        assert!(RustFfiProcessor::is_valid_export_symbol("foo_bar"));
        assert!(!RustFfiProcessor::is_valid_export_symbol(""));
        assert!(!RustFfiProcessor::is_valid_export_symbol("2foo"));
        assert!(!RustFfiProcessor::is_valid_export_symbol("foo?"));
    }

    #[test]
    fn is_valid_rust_symbol_path_accepts_simple_and_module_paths() {
        assert!(RustFfiProcessor::is_valid_rust_symbol_path("answer"));
        assert!(RustFfiProcessor::is_valid_rust_symbol_path("math_core::answer"));
        assert!(RustFfiProcessor::is_valid_rust_symbol_path("Math::answer"));
        assert!(!RustFfiProcessor::is_valid_rust_symbol_path(""));
        assert!(!RustFfiProcessor::is_valid_rust_symbol_path("Math::"));
        assert!(!RustFfiProcessor::is_valid_rust_symbol_path("::answer"));
        assert!(!RustFfiProcessor::is_valid_rust_symbol_path("bad symbol"));
        assert!(!RustFfiProcessor::is_valid_rust_symbol_path("2math::answer"));
    }

    #[test]
    fn resolve_registry_lib_src_errors_when_registry_package_missing() {
        let metadata = CargoMetadata {
            packages: vec![CargoPackage {
                name: "demo-math".to_string(),
                version: "0.1.0".to_string(),
                source: Some("path+file:///tmp/demo-math".to_string()),
                targets: vec![CargoTarget {
                    kind: vec!["lib".to_string()],
                    src_path: "/tmp/local/src/lib.rs".to_string(),
                }],
            }],
        };

        let err = RustFfiProcessor::resolve_registry_lib_src(&metadata, "demo-math")
            .expect_err("should fail when only non-registry package exists");
        assert!(err.contains("Registry dependency 'demo-math' was not found"));
    }

    #[test]
    fn resolve_registry_lib_src_errors_when_registry_package_has_no_lib_target() {
        let metadata = CargoMetadata {
            packages: vec![CargoPackage {
                name: "demo-math".to_string(),
                version: "0.3.0".to_string(),
                source: Some("registry+https://github.com/rust-lang/crates.io-index".to_string()),
                targets: vec![CargoTarget {
                    kind: vec!["bin".to_string()],
                    src_path: "/tmp/registry/src/main.rs".to_string(),
                }],
            }],
        };

        let err = RustFfiProcessor::resolve_registry_lib_src(&metadata, "demo-math")
            .expect_err("should fail when registry package has no lib target");
        assert!(err.contains("Registry dependency 'demo-math' (resolved 0.3.0) has no lib target"));
    }

    #[test]
    fn retain_supported_ffi_functions_filters_unsupported_signatures() {
        let functions = vec![
            FunctionInfo {
                name: "ok_add".to_string(),
                params: vec![
                    clorus_ffi_gen::ParamInfo {
                        name: "a".to_string(),
                        type_name: "f64".to_string(),
                    },
                    clorus_ffi_gen::ParamInfo {
                        name: "b".to_string(),
                        type_name: "f64".to_string(),
                    },
                ],
                return_type: "f64".to_string(),
            },
            FunctionInfo {
                name: "ok_i64".to_string(),
                params: vec![clorus_ffi_gen::ParamInfo {
                    name: "n".to_string(),
                    type_name: "i64".to_string(),
                }],
                return_type: "i64".to_string(),
            },
            FunctionInfo {
                name: "bad_vec".to_string(),
                params: vec![clorus_ffi_gen::ParamInfo {
                    name: "items".to_string(),
                    type_name: "Vec<u8>".to_string(),
                }],
                return_type: "Vec<u8>".to_string(),
            },
        ];

        let filtered = RustFfiProcessor::retain_supported_ffi_functions(functions, false);
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].name, "ok_add");
        assert_eq!(filtered[1].name, "ok_i64");
    }

    #[test]
    fn retain_supported_ffi_functions_accepts_extended_numeric_types() {
        let functions = vec![
            FunctionInfo {
                name: "ok_f32".to_string(),
                params: vec![clorus_ffi_gen::ParamInfo {
                    name: "x".to_string(),
                    type_name: "f32".to_string(),
                }],
                return_type: "f32".to_string(),
            },
            FunctionInfo {
                name: "ok_u32".to_string(),
                params: vec![clorus_ffi_gen::ParamInfo {
                    name: "x".to_string(),
                    type_name: "u32".to_string(),
                }],
                return_type: "u32".to_string(),
            },
            FunctionInfo {
                name: "ok_u64".to_string(),
                params: vec![clorus_ffi_gen::ParamInfo {
                    name: "x".to_string(),
                    type_name: "u64".to_string(),
                }],
                return_type: "u64".to_string(),
            },
            FunctionInfo {
                name: "ok_usize".to_string(),
                params: vec![clorus_ffi_gen::ParamInfo {
                    name: "x".to_string(),
                    type_name: "usize".to_string(),
                }],
                return_type: "usize".to_string(),
            },
            FunctionInfo {
                name: "bad_vec".to_string(),
                params: vec![clorus_ffi_gen::ParamInfo {
                    name: "x".to_string(),
                    type_name: "Vec<u8>".to_string(),
                }],
                return_type: "Vec<u8>".to_string(),
            },
        ];

        let filtered = RustFfiProcessor::retain_supported_ffi_functions(functions, false);
        let names: Vec<String> = filtered.into_iter().map(|f| f.name).collect();
        assert_eq!(names, vec!["ok_f32", "ok_u32", "ok_u64", "ok_usize"]);
    }

    #[test]
    fn retain_supported_ffi_functions_accepts_narrow_integer_types() {
        let functions = vec![
            FunctionInfo {
                name: "ok_i8".to_string(),
                params: vec![clorus_ffi_gen::ParamInfo {
                    name: "x".to_string(),
                    type_name: "i8".to_string(),
                }],
                return_type: "i8".to_string(),
            },
            FunctionInfo {
                name: "ok_u8".to_string(),
                params: vec![clorus_ffi_gen::ParamInfo {
                    name: "x".to_string(),
                    type_name: "u8".to_string(),
                }],
                return_type: "u8".to_string(),
            },
            FunctionInfo {
                name: "ok_i16".to_string(),
                params: vec![clorus_ffi_gen::ParamInfo {
                    name: "x".to_string(),
                    type_name: "i16".to_string(),
                }],
                return_type: "i16".to_string(),
            },
            FunctionInfo {
                name: "ok_u16".to_string(),
                params: vec![clorus_ffi_gen::ParamInfo {
                    name: "x".to_string(),
                    type_name: "u16".to_string(),
                }],
                return_type: "u16".to_string(),
            },
        ];

        let filtered = RustFfiProcessor::retain_supported_ffi_functions(functions, false);
        let names: Vec<String> = filtered.into_iter().map(|f| f.name).collect();
        assert_eq!(names, vec!["ok_i8", "ok_u8", "ok_i16", "ok_u16"]);
    }

    #[test]
    fn retain_supported_ffi_functions_accepts_core_supported_types() {
        let functions = vec![
            FunctionInfo {
                name: "ok_bool".to_string(),
                params: vec![clorus_ffi_gen::ParamInfo {
                    name: "x".to_string(),
                    type_name: "bool".to_string(),
                }],
                return_type: "bool".to_string(),
            },
            FunctionInfo {
                name: "ok_string".to_string(),
                params: vec![clorus_ffi_gen::ParamInfo {
                    name: "s".to_string(),
                    type_name: "String".to_string(),
                }],
                return_type: "String".to_string(),
            },
            FunctionInfo {
                name: "ok_ptr".to_string(),
                params: vec![],
                return_type: "*mut u8".to_string(),
            },
            FunctionInfo {
                name: "ok_void".to_string(),
                params: vec![clorus_ffi_gen::ParamInfo {
                    name: "p".to_string(),
                    type_name: "*mut u8".to_string(),
                }],
                return_type: "()".to_string(),
            },
        ];

        let filtered = RustFfiProcessor::retain_supported_ffi_functions(functions, false);
        let names: Vec<String> = filtered.into_iter().map(|f| f.name).collect();
        assert_eq!(names, vec!["ok_bool", "ok_string", "ok_ptr", "ok_void"]);
    }

    #[test]
    fn collect_unsupported_signature_details_reports_reason() {
        let functions = vec![
            FunctionInfo {
                name: "bad_param".to_string(),
                params: vec![clorus_ffi_gen::ParamInfo {
                    name: "bytes".to_string(),
                    type_name: "Vec<u8>".to_string(),
                }],
                return_type: "()".to_string(),
            },
            FunctionInfo {
                name: "bad_ret".to_string(),
                params: vec![],
                return_type: "Vec<u8>".to_string(),
            },
        ];

        let details = RustFfiProcessor::collect_unsupported_signature_details(&functions);
        assert_eq!(details.len(), 2);
        assert!(details[0].contains("unsupported param"));
        assert!(details[1].contains("unsupported return type"));
    }

    #[test]
    fn resolve_interface_path_auto_prefers_clri() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("clorus_rustffi_iface_{}", unique));
        let iface_dir = root.join("interfaces");
        std::fs::create_dir_all(&iface_dir).expect("create interfaces dir");
        std::fs::write(iface_dir.join("libm.clri"), "(interface libm)").expect("write clri");

        let resolved = with_cwd(&root, || {
            RustFfiProcessor::resolve_interface_path("libm", crate::manifest::InterfaceSpec::Auto)
                .expect("auto resolve should find .clri")
        });
        let _ = std::fs::remove_dir_all(root);

        assert_eq!(resolved, "interfaces/libm.clri");
    }

    #[test]
    fn resolve_interface_path_auto_reports_clear_error_when_missing() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("clorus_rustffi_iface_missing_{}", unique));
        std::fs::create_dir_all(&root).expect("create temp root");

        let err = with_cwd(&root, || {
            RustFfiProcessor::resolve_interface_path(
                "missing-lib",
                crate::manifest::InterfaceSpec::Auto,
            )
            .expect_err("auto resolve should fail")
        });
        let _ = std::fs::remove_dir_all(root);

        assert!(err.contains("Interface auto-discovery failed"));
        assert!(err.contains("interfaces/missing-lib.clri"));
    }

    #[test]
    fn resolve_interface_path_explicit_rejects_unknown_extension() {
        let err = RustFfiProcessor::resolve_interface_path(
            "demo-lib",
            crate::manifest::InterfaceSpec::Path("interfaces/demo-lib.txt".to_string()),
        )
        .expect_err("expected unsupported extension error");
        assert!(err.contains("Unsupported interface file extension"));
        assert!(err.contains(".clri"));
        assert!(err.contains(".clorus-ffi"));
    }

    #[test]
    fn resolve_interface_path_explicit_rejects_empty_path() {
        let err = RustFfiProcessor::resolve_interface_path(
            "demo-lib",
            crate::manifest::InterfaceSpec::Path("   ".to_string()),
        )
        .expect_err("expected empty path error");
        assert!(err.contains("cannot be empty"));
    }

    #[test]
    fn likely_impl_method_only_api_detects_impl_patterns() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("clorus_impl_detect_{}", unique));
        std::fs::create_dir_all(&root).expect("create temp root");
        let src = root.join("lib.rs");
        std::fs::write(
            &src,
            r#"
pub struct Foo;
impl Foo {
    pub fn new() -> Self { Foo }
}
"#,
        )
        .expect("write source");

        let detected = RustFfiProcessor::likely_impl_method_only_api(&src);
        let _ = std::fs::remove_dir_all(&root);
        assert!(detected);
    }

    #[test]
    fn generate_wrapper_lib_rs_from_interface_supports_rust_symbol_override() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("clorus_iface_wrap_{}", unique));
        std::fs::create_dir_all(root.join("src")).expect("create temp src");

        let bindings = vec![InterfaceBinding {
            exposed_name: "new_point".to_string(),
            rust_symbol: "Point::new".to_string(),
            params: vec![
                clorus_ffi_gen::ParamInfo {
                    name: "x".to_string(),
                    type_name: "f64".to_string(),
                },
                clorus_ffi_gen::ParamInfo {
                    name: "y".to_string(),
                    type_name: "f64".to_string(),
                },
            ],
            return_type: "()".to_string(),
        }];

        RustFfiProcessor::generate_wrapper_lib_rs_from_interface(&root, "demo-lib", &bindings)
            .expect("generate wrapper");

        let generated =
            std::fs::read_to_string(root.join("src/lib.rs")).expect("read generated wrapper");
        let _ = std::fs::remove_dir_all(&root);

        assert!(generated.contains("clorus_new_point"));
        assert!(generated.contains("Point::new"));
    }

    #[test]
    fn generate_wrapper_lib_rs_from_interface_handles_pointer_sized_ints() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("clorus_iface_wrap_ints_{}", unique));
        std::fs::create_dir_all(root.join("src")).expect("create temp src");

        let bindings = vec![
            InterfaceBinding {
                exposed_name: "take_isize".to_string(),
                rust_symbol: "take_isize".to_string(),
                params: vec![clorus_ffi_gen::ParamInfo {
                    name: "n".to_string(),
                    type_name: "isize".to_string(),
                }],
                return_type: "isize".to_string(),
            },
            InterfaceBinding {
                exposed_name: "take_usize".to_string(),
                rust_symbol: "take_usize".to_string(),
                params: vec![clorus_ffi_gen::ParamInfo {
                    name: "n".to_string(),
                    type_name: "usize".to_string(),
                }],
                return_type: "usize".to_string(),
            },
        ];

        RustFfiProcessor::generate_wrapper_lib_rs_from_interface(&root, "demo-lib", &bindings)
            .expect("generate wrapper");

        let generated =
            std::fs::read_to_string(root.join("src/lib.rs")).expect("read generated wrapper");
        let _ = std::fs::remove_dir_all(&root);

        // ABI carrier types
        assert!(generated.contains("fn clorus_take_isize(n: i64) -> i64"));
        assert!(generated.contains("fn clorus_take_usize(n: u64) -> u64"));
        // Rust-side conversions
        assert!(generated.contains("let n_rust = n as isize;"));
        assert!(generated.contains("let n_rust = n as usize;"));
    }

    #[test]
    fn process_dependencies_e2e_local_path_interface_extended_numeric_types() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("clorus_rustffi_e2e_{}", unique));
        let dep_dir = root.join("demo-math");
        let iface_dir = root.join("interfaces");
        std::fs::create_dir_all(dep_dir.join("src")).expect("create dep src");
        std::fs::create_dir_all(&iface_dir).expect("create interfaces dir");

        std::fs::write(
            dep_dir.join("Cargo.toml"),
            r#"[package]
name = "demo-math"
version = "0.1.0"
edition = "2021"
"#,
        )
        .expect("write dep cargo");

        std::fs::write(
            dep_dir.join("src/lib.rs"),
            r#"
pub fn id_f32(x: f32) -> f32 { x }
pub fn id_u64(x: u64) -> u64 { x }
pub fn id_usize(x: usize) -> usize { x }
"#,
        )
        .expect("write dep lib");

        std::fs::write(
            iface_dir.join("demo-math.clri"),
            r#"(interface demo-math
  (fn id-f32 [x :f32] :f32)
  (fn id-u64 [x :u64] :u64)
  (fn id-usize [x :usize] :usize)
)"#,
        )
        .expect("write interface");

        let manifest: Manifest = toml::from_str(
            r#"[package]
name = "demo"
version = "0.1.0"

[build]
entry = "src/main.clrs"

[rust-dependencies]
demo-math = { path = "demo-math", interface = true }
"#,
        )
        .expect("parse manifest");

        let processed = with_cwd(&root, || {
            RustFfiProcessor::process_dependencies(&manifest, false)
                .expect("process dependencies")
        });

        assert_eq!(processed.libraries.len(), 1);
        let lib = &processed.libraries[0];
        assert_eq!(lib.name, "demo_math_ffi");
        let static_lib = if lib.static_lib_path.is_absolute() {
            lib.static_lib_path.clone()
        } else {
            root.join(&lib.static_lib_path)
        };
        assert!(static_lib.exists(), "expected static library at {}", static_lib.display());
        assert_eq!(lib.functions.len(), 3);
        assert_eq!(lib.functions[0].return_type, "f32");
        assert_eq!(lib.functions[1].return_type, "u64");
        assert_eq!(lib.functions[2].return_type, "usize");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn process_dependencies_e2e_local_path_interface_narrow_integer_types() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("clorus_rustffi_e2e_narrow_{}", unique));
        let dep_dir = root.join("demo-narrow");
        let iface_dir = root.join("interfaces");
        std::fs::create_dir_all(dep_dir.join("src")).expect("create dep src");
        std::fs::create_dir_all(&iface_dir).expect("create interfaces dir");

        std::fs::write(
            dep_dir.join("Cargo.toml"),
            r#"[package]
name = "demo-narrow"
version = "0.1.0"
edition = "2021"
"#,
        )
        .expect("write dep cargo");

        std::fs::write(
            dep_dir.join("src/lib.rs"),
            r#"
pub fn id_i8(x: i8) -> i8 { x }
pub fn id_u8(x: u8) -> u8 { x }
pub fn id_i16(x: i16) -> i16 { x }
pub fn id_u16(x: u16) -> u16 { x }
"#,
        )
        .expect("write dep lib");

        std::fs::write(
            iface_dir.join("demo-narrow.clri"),
            r#"(interface demo-narrow
  (fn id-i8 [x :i8] :i8)
  (fn id-u8 [x :u8] :u8)
  (fn id-i16 [x :i16] :i16)
  (fn id-u16 [x :u16] :u16)
)"#,
        )
        .expect("write interface");

        let manifest: Manifest = toml::from_str(
            r#"[package]
name = "demo"
version = "0.1.0"

[build]
entry = "src/main.clrs"

[rust-dependencies]
demo-narrow = { path = "demo-narrow", interface = true }
"#,
        )
        .expect("parse manifest");

        let processed = with_cwd(&root, || {
            RustFfiProcessor::process_dependencies(&manifest, false)
                .expect("process dependencies")
        });

        assert_eq!(processed.libraries.len(), 1);
        let lib = &processed.libraries[0];
        assert_eq!(lib.name, "demo_narrow_ffi");
        let static_lib = if lib.static_lib_path.is_absolute() {
            lib.static_lib_path.clone()
        } else {
            root.join(&lib.static_lib_path)
        };
        assert!(static_lib.exists(), "expected static library at {}", static_lib.display());
        assert_eq!(lib.functions.len(), 4);
        assert_eq!(lib.functions[0].return_type, "i8");
        assert_eq!(lib.functions[1].return_type, "u8");
        assert_eq!(lib.functions[2].return_type, "i16");
        assert_eq!(lib.functions[3].return_type, "u16");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn process_dependencies_e2e_local_path_interface_bool_and_string_types() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("clorus_rustffi_e2e_bool_string_{}", unique));
        let dep_dir = root.join("demo-text");
        let iface_dir = root.join("interfaces");
        std::fs::create_dir_all(dep_dir.join("src")).expect("create dep src");
        std::fs::create_dir_all(&iface_dir).expect("create interfaces dir");

        std::fs::write(
            dep_dir.join("Cargo.toml"),
            r#"[package]
name = "demo-text"
version = "0.1.0"
edition = "2021"
"#,
        )
        .expect("write dep cargo");

        std::fs::write(
            dep_dir.join("src/lib.rs"),
            r#"
pub fn flip_bool(x: bool) -> bool { !x }
pub fn echo_string(s: String) -> String { s }
"#,
        )
        .expect("write dep lib");

        std::fs::write(
            iface_dir.join("demo-text.clri"),
            r#"(interface demo-text
  (fn flip-bool [x :bool] :bool)
  (fn echo-string [s :string] :string)
)"#,
        )
        .expect("write interface");

        let manifest: Manifest = toml::from_str(
            r#"[package]
name = "demo"
version = "0.1.0"

[build]
entry = "src/main.clrs"

[rust-dependencies]
demo-text = { path = "demo-text", interface = true }
"#,
        )
        .expect("parse manifest");

        let processed = with_cwd(&root, || {
            RustFfiProcessor::process_dependencies(&manifest, false)
                .expect("process dependencies")
        });

        assert_eq!(processed.libraries.len(), 1);
        let lib = &processed.libraries[0];
        assert_eq!(lib.name, "demo_text_ffi");
        assert_eq!(lib.functions.len(), 2);
        assert_eq!(lib.functions[0].return_type, "bool");
        assert_eq!(lib.functions[1].return_type, "String");
        assert_eq!(lib.functions[1].params[0].type_name, "String");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn process_dependencies_e2e_local_path_interface_with_rust_symbol_override() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("clorus_rustffi_e2e_override_{}", unique));
        let dep_dir = root.join("impl-only-lib");
        let iface_dir = root.join("interfaces");
        std::fs::create_dir_all(dep_dir.join("src")).expect("create dep src");
        std::fs::create_dir_all(&iface_dir).expect("create interfaces dir");

        std::fs::write(
            dep_dir.join("Cargo.toml"),
            r#"[package]
name = "impl-only-lib"
version = "0.1.0"
edition = "2021"
"#,
        )
        .expect("write dep cargo");

        std::fs::write(
            dep_dir.join("src/lib.rs"),
            r#"
pub struct Math;
impl Math {
    pub fn forty_two() -> u64 { 42 }
}
"#,
        )
        .expect("write dep lib");

        std::fs::write(
            iface_dir.join("impl-only-lib.clri"),
            r#"(interface impl-only-lib
  (fn forty-two [] :u64 :rust "Math::forty_two")
)"#,
        )
        .expect("write interface");

        let manifest: Manifest = toml::from_str(
            r#"[package]
name = "demo"
version = "0.1.0"

[build]
entry = "src/main.clrs"

[rust-dependencies]
impl-only-lib = { path = "impl-only-lib", interface = true }
"#,
        )
        .expect("parse manifest");

        let processed = with_cwd(&root, || {
            RustFfiProcessor::process_dependencies(&manifest, false)
                .expect("process dependencies")
        });

        assert_eq!(processed.libraries.len(), 1);
        let lib = &processed.libraries[0];
        assert_eq!(lib.name, "impl_only_lib_ffi");
        let static_lib = if lib.static_lib_path.is_absolute() {
            lib.static_lib_path.clone()
        } else {
            root.join(&lib.static_lib_path)
        };
        assert!(static_lib.exists(), "expected static library at {}", static_lib.display());
        assert_eq!(lib.functions.len(), 1);
        assert_eq!(lib.functions[0].name, "forty_two");
        assert_eq!(lib.functions[0].return_type, "u64");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn process_dependencies_e2e_interface_auto_missing_reports_clear_paths() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("clorus_rustffi_e2e_missing_iface_{}", unique));
        let dep_dir = root.join("missing-iface-lib");
        std::fs::create_dir_all(dep_dir.join("src")).expect("create dep src");

        std::fs::write(
            dep_dir.join("Cargo.toml"),
            r#"[package]
name = "missing-iface-lib"
version = "0.1.0"
edition = "2021"
"#,
        )
        .expect("write dep cargo");

        std::fs::write(dep_dir.join("src/lib.rs"), "pub fn ping() -> i32 { 1 }\n")
            .expect("write dep lib");

        let manifest: Manifest = toml::from_str(
            r#"[package]
name = "demo"
version = "0.1.0"

[build]
entry = "src/main.clrs"

[rust-dependencies]
missing-iface-lib = { path = "missing-iface-lib", interface = true }
"#,
        )
        .expect("parse manifest");

        let err = with_cwd(&root, || match RustFfiProcessor::process_dependencies(&manifest, false) {
            Ok(_) => panic!("expected interface auto-discovery failure"),
            Err(e) => e,
        });

        assert!(err.contains("Interface auto-discovery failed"));
        assert!(err.contains("interfaces/missing-iface-lib.clri"));
        assert!(err.contains("interfaces/missing-iface-lib.clorus-ffi"));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn process_dependencies_e2e_explicit_interface_missing_reports_path() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("clorus_rustffi_e2e_missing_explicit_iface_{}", unique));
        let dep_dir = root.join("missing-explicit-iface-lib");
        std::fs::create_dir_all(dep_dir.join("src")).expect("create dep src");

        std::fs::write(
            dep_dir.join("Cargo.toml"),
            r#"[package]
name = "missing-explicit-iface-lib"
version = "0.1.0"
edition = "2021"
"#,
        )
        .expect("write dep cargo");

        std::fs::write(dep_dir.join("src/lib.rs"), "pub fn ping() -> i32 { 1 }\n")
            .expect("write dep lib");

        let manifest: Manifest = toml::from_str(
            r#"[package]
name = "demo"
version = "0.1.0"

[build]
entry = "src/main.clrs"

[rust-dependencies]
missing-explicit-iface-lib = { path = "missing-explicit-iface-lib", interface = "interfaces/not-there.clri" }
"#,
        )
        .expect("parse manifest");

        let err = with_cwd(&root, || match RustFfiProcessor::process_dependencies(&manifest, false) {
            Ok(_) => panic!("expected explicit interface missing failure"),
            Err(e) => e,
        });

        assert!(err.contains("Interface file not found"));
        assert!(err.contains("interfaces/not-there.clri"));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn process_dependencies_e2e_explicit_interface_directory_path_reports_clear_error() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "clorus_rustffi_e2e_explicit_interface_dir_{}",
            unique
        ));
        let dep_dir = root.join("dir-iface-lib");
        let iface_dir = root.join("interfaces");
        let bad_iface_dir = iface_dir.join("dir-iface-lib.clri");
        std::fs::create_dir_all(dep_dir.join("src")).expect("create dep src");
        std::fs::create_dir_all(&bad_iface_dir).expect("create bad interface dir");

        std::fs::write(
            dep_dir.join("Cargo.toml"),
            r#"[package]
name = "dir-iface-lib"
version = "0.1.0"
edition = "2021"
"#,
        )
        .expect("write dep cargo");

        std::fs::write(dep_dir.join("src/lib.rs"), "pub fn ping() -> i32 { 1 }\n")
            .expect("write dep lib");

        let manifest: Manifest = toml::from_str(
            r#"[package]
name = "demo"
version = "0.1.0"

[build]
entry = "src/main.clrs"

[rust-dependencies]
dir-iface-lib = { path = "dir-iface-lib", interface = "interfaces/dir-iface-lib.clri" }
"#,
        )
        .expect("parse manifest");

        let result = with_cwd(&root, || RustFfiProcessor::process_dependencies(&manifest, false));
        let err = match result {
            Ok(_) => panic!("expected explicit interface directory-path failure"),
            Err(e) => e,
        };
        assert!(err.contains("Interface path points to a directory"));
        assert!(err.contains("interfaces/dir-iface-lib.clri"));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn process_dependencies_e2e_local_path_auto_parse_filters_unsupported_signatures() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("clorus_rustffi_e2e_autoparse_{}", unique));
        let dep_dir = root.join("auto-parse-lib");
        std::fs::create_dir_all(dep_dir.join("src")).expect("create dep src");

        std::fs::write(
            dep_dir.join("Cargo.toml"),
            r#"[package]
name = "auto-parse-lib"
version = "0.1.0"
edition = "2021"
"#,
        )
        .expect("write dep cargo");

        std::fs::write(
            dep_dir.join("src/lib.rs"),
            r#"
pub fn id_u64(x: u64) -> u64 { x }
pub fn id_usize(x: usize) -> usize { x }
pub fn id_f32(x: f32) -> f32 { x }
pub fn unsupported_vec(xs: Vec<u8>) -> Vec<u8> { xs }
"#,
        )
        .expect("write dep lib");

        let manifest: Manifest = toml::from_str(
            r#"[package]
name = "demo"
version = "0.1.0"

[build]
entry = "src/main.clrs"

[rust-dependencies]
auto-parse-lib = { path = "auto-parse-lib" }
"#,
        )
        .expect("parse manifest");

        let processed = with_cwd(&root, || {
            RustFfiProcessor::process_dependencies(&manifest, false)
                .expect("process dependencies")
        });

        assert_eq!(processed.libraries.len(), 1);
        let lib = &processed.libraries[0];
        assert_eq!(lib.name, "auto_parse_lib_ffi");
        let static_lib = if lib.static_lib_path.is_absolute() {
            lib.static_lib_path.clone()
        } else {
            root.join(&lib.static_lib_path)
        };
        assert!(static_lib.exists(), "expected static library at {}", static_lib.display());

        let names: Vec<&str> = lib.functions.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"id_u64"));
        assert!(names.contains(&"id_usize"));
        assert!(names.contains(&"id_f32"));
        assert!(!names.contains(&"unsupported_vec"));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn process_dependencies_e2e_local_path_auto_parse_supports_bool_string_and_pointer() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("clorus_rustffi_e2e_autoparse_mixed_{}", unique));
        let dep_dir = root.join("auto-parse-mixed-lib");
        std::fs::create_dir_all(dep_dir.join("src")).expect("create dep src");

        std::fs::write(
            dep_dir.join("Cargo.toml"),
            r#"[package]
name = "auto-parse-mixed-lib"
version = "0.1.0"
edition = "2021"
"#,
        )
        .expect("write dep cargo");

        std::fs::write(
            dep_dir.join("src/lib.rs"),
            r#"
pub fn flip_bool(x: bool) -> bool { !x }
pub fn id_string(s: String) -> String { s }
pub fn id_ptr(p: *mut u8) -> *mut u8 { p }
pub fn unsupported_vec(xs: Vec<u8>) -> Vec<u8> { xs }
"#,
        )
        .expect("write dep lib");

        let manifest: Manifest = toml::from_str(
            r#"[package]
name = "demo"
version = "0.1.0"

[build]
entry = "src/main.clrs"

[rust-dependencies]
auto-parse-mixed-lib = { path = "auto-parse-mixed-lib" }
"#,
        )
        .expect("parse manifest");

        let processed = with_cwd(&root, || {
            RustFfiProcessor::process_dependencies(&manifest, false)
                .expect("process dependencies")
        });

        assert_eq!(processed.libraries.len(), 1);
        let lib = &processed.libraries[0];
        assert_eq!(lib.name, "auto_parse_mixed_lib_ffi");

        let names: Vec<&str> = lib.functions.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"flip_bool"));
        assert!(names.contains(&"id_string"));
        assert!(names.contains(&"id_ptr"));
        assert!(!names.contains(&"unsupported_vec"));

        let id_string = lib
            .functions
            .iter()
            .find(|f| f.name == "id_string")
            .expect("id_string function should exist");
        assert_eq!(id_string.params[0].type_name, "String");
        assert_eq!(id_string.return_type, "String");

        let id_ptr = lib
            .functions
            .iter()
            .find(|f| f.name == "id_ptr")
            .expect("id_ptr function should exist");
        assert_eq!(id_ptr.params[0].type_name, "*mut u8");
        assert_eq!(id_ptr.return_type, "*mut u8");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn process_dependencies_e2e_local_path_auto_parse_rejects_unsupported_pointer_signatures() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("clorus_rustffi_e2e_autoparse_bad_ptr_{}", unique));
        let dep_dir = root.join("auto-parse-bad-ptr-lib");
        std::fs::create_dir_all(dep_dir.join("src")).expect("create dep src");

        std::fs::write(
            dep_dir.join("Cargo.toml"),
            r#"[package]
name = "auto-parse-bad-ptr-lib"
version = "0.1.0"
edition = "2021"
"#,
        )
        .expect("write dep cargo");

        std::fs::write(
            dep_dir.join("src/lib.rs"),
            r#"
pub fn only_bad_ptr(p: *mut i32) -> *mut i32 { p }
"#,
        )
        .expect("write dep lib");

        let manifest: Manifest = toml::from_str(
            r#"[package]
name = "demo"
version = "0.1.0"

[build]
entry = "src/main.clrs"

[rust-dependencies]
auto-parse-bad-ptr-lib = { path = "auto-parse-bad-ptr-lib" }
"#,
        )
        .expect("parse manifest");

        let err = with_cwd(&root, || match RustFfiProcessor::process_dependencies(&manifest, false)
        {
            Ok(_) => panic!("expected unsupported pointer signature rejection"),
            Err(e) => e,
        });
        assert!(
            err.contains("Wrapper crate build failed"),
            "expected wrapper build failure, got: {}",
            err
        );
        assert!(
            err.contains("supports raw pointers only as `*mut u8`"),
            "expected pointer support hint, got: {}",
            err
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn process_dependencies_e2e_local_path_explicit_legacy_interface_path() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("clorus_rustffi_e2e_legacy_iface_{}", unique));
        let dep_dir = root.join("legacy-iface-lib");
        let custom_iface_dir = root.join("custom-ifaces");
        std::fs::create_dir_all(dep_dir.join("src")).expect("create dep src");
        std::fs::create_dir_all(&custom_iface_dir).expect("create custom interface dir");

        std::fs::write(
            dep_dir.join("Cargo.toml"),
            r#"[package]
name = "legacy-iface-lib"
version = "0.1.0"
edition = "2021"
"#,
        )
        .expect("write dep cargo");

        std::fs::write(
            dep_dir.join("src/lib.rs"),
            r#"
pub fn ping() -> i32 { 7 }
"#,
        )
        .expect("write dep lib");

        std::fs::write(
            custom_iface_dir.join("legacy-iface-lib.clorus-ffi"),
            r#"(interface legacy-iface-lib
  (fn ping [] :i32)
)"#,
        )
        .expect("write legacy interface");

        let manifest: Manifest = toml::from_str(
            r#"[package]
name = "demo"
version = "0.1.0"

[build]
entry = "src/main.clrs"

[rust-dependencies]
legacy-iface-lib = { path = "legacy-iface-lib", interface = "custom-ifaces/legacy-iface-lib.clorus-ffi" }
"#,
        )
        .expect("parse manifest");

        let processed = with_cwd(&root, || {
            RustFfiProcessor::process_dependencies(&manifest, false)
                .expect("process dependencies")
        });

        assert_eq!(processed.libraries.len(), 1);
        let lib = &processed.libraries[0];
        assert_eq!(lib.name, "legacy_iface_lib_ffi");
        let static_lib = if lib.static_lib_path.is_absolute() {
            lib.static_lib_path.clone()
        } else {
            root.join(&lib.static_lib_path)
        };
        assert!(static_lib.exists(), "expected static library at {}", static_lib.display());
        assert_eq!(lib.functions.len(), 1);
        assert_eq!(lib.functions[0].name, "ping");
        assert_eq!(lib.functions[0].return_type, "i32");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn process_dependencies_e2e_explicit_legacy_interface_with_rust_symbol_override() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "clorus_rustffi_e2e_explicit_legacy_override_{}",
            unique
        ));
        let dep_dir = root.join("legacy-override-lib");
        let iface_dir = root.join("interfaces");
        std::fs::create_dir_all(dep_dir.join("src")).expect("create dep src");
        std::fs::create_dir_all(&iface_dir).expect("create interfaces dir");

        std::fs::write(
            dep_dir.join("Cargo.toml"),
            r#"[package]
name = "legacy-override-lib"
version = "0.1.0"
edition = "2021"
"#,
        )
        .expect("write dep cargo");

        std::fs::write(
            dep_dir.join("src/lib.rs"),
            r#"
pub struct Math;
impl Math {
    pub fn answer() -> u64 { 42 }
}
"#,
        )
        .expect("write dep lib");

        std::fs::write(
            iface_dir.join("legacy-override-lib.clorus-ffi"),
            r#"(interface legacy-override-lib
  (fn forty-two [] :u64 :rust "Math::answer")
)"#,
        )
        .expect("write legacy interface");

        let manifest: Manifest = toml::from_str(
            r#"[package]
name = "demo"
version = "0.1.0"

[build]
entry = "src/main.clrs"

[rust-dependencies]
legacy-override-lib = { path = "legacy-override-lib", interface = "interfaces/legacy-override-lib.clorus-ffi" }
"#,
        )
        .expect("parse manifest");

        let processed = with_cwd(&root, || {
            RustFfiProcessor::process_dependencies(&manifest, false)
                .expect("process dependencies")
        });

        assert_eq!(processed.libraries.len(), 1);
        let lib = &processed.libraries[0];
        assert_eq!(lib.name, "legacy_override_lib_ffi");
        assert_eq!(lib.functions.len(), 1);
        assert_eq!(lib.functions[0].name, "forty_two");
        assert_eq!(lib.functions[0].return_type, "u64");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn process_dependencies_e2e_local_path_explicit_clri_interface_path() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("clorus_rustffi_e2e_explicit_clri_{}", unique));
        let dep_dir = root.join("explicit-clri-lib");
        let custom_iface_dir = root.join("custom-ifaces");
        std::fs::create_dir_all(dep_dir.join("src")).expect("create dep src");
        std::fs::create_dir_all(&custom_iface_dir).expect("create custom interface dir");

        std::fs::write(
            dep_dir.join("Cargo.toml"),
            r#"[package]
name = "explicit-clri-lib"
version = "0.1.0"
edition = "2021"
"#,
        )
        .expect("write dep cargo");

        std::fs::write(
            dep_dir.join("src/lib.rs"),
            r#"
pub fn ping() -> i32 { 13 }
"#,
        )
        .expect("write dep lib");

        std::fs::write(
            custom_iface_dir.join("explicit-clri-lib.clri"),
            r#"(interface explicit-clri-lib
  (fn ping [] :i32)
)"#,
        )
        .expect("write clri interface");

        let manifest: Manifest = toml::from_str(
            r#"[package]
name = "demo"
version = "0.1.0"

[build]
entry = "src/main.clrs"

[rust-dependencies]
explicit-clri-lib = { path = "explicit-clri-lib", interface = "custom-ifaces/explicit-clri-lib.clri" }
"#,
        )
        .expect("parse manifest");

        let processed = with_cwd(&root, || {
            RustFfiProcessor::process_dependencies(&manifest, false)
                .expect("process dependencies")
        });

        assert_eq!(processed.libraries.len(), 1);
        let lib = &processed.libraries[0];
        assert_eq!(lib.name, "explicit_clri_lib_ffi");
        let static_lib = if lib.static_lib_path.is_absolute() {
            lib.static_lib_path.clone()
        } else {
            root.join(&lib.static_lib_path)
        };
        assert!(static_lib.exists(), "expected static library at {}", static_lib.display());
        assert_eq!(lib.functions.len(), 1);
        assert_eq!(lib.functions[0].name, "ping");
        assert_eq!(lib.functions[0].return_type, "i32");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn process_dependencies_e2e_explicit_clri_with_rust_symbol_override() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("clorus_rustffi_e2e_explicit_clri_override_{}", unique));
        let dep_dir = root.join("explicit-override-lib");
        let iface_dir = root.join("interfaces");
        std::fs::create_dir_all(dep_dir.join("src")).expect("create dep src");
        std::fs::create_dir_all(&iface_dir).expect("create interfaces dir");

        std::fs::write(
            dep_dir.join("Cargo.toml"),
            r#"[package]
name = "explicit-override-lib"
version = "0.1.0"
edition = "2021"
"#,
        )
        .expect("write dep cargo");

        std::fs::write(
            dep_dir.join("src/lib.rs"),
            r#"
pub struct Math;
impl Math {
    pub fn answer() -> u64 { 42 }
}
"#,
        )
        .expect("write dep lib");

        std::fs::write(
            iface_dir.join("explicit-override-lib.clri"),
            r#"(interface explicit-override-lib
  (fn forty-two [] :u64 :rust "Math::answer")
)"#,
        )
        .expect("write interface");

        let manifest: Manifest = toml::from_str(
            r#"[package]
name = "demo"
version = "0.1.0"

[build]
entry = "src/main.clrs"

[rust-dependencies]
explicit-override-lib = { path = "explicit-override-lib", interface = "interfaces/explicit-override-lib.clri" }
"#,
        )
        .expect("parse manifest");

        let processed = with_cwd(&root, || {
            RustFfiProcessor::process_dependencies(&manifest, false)
                .expect("process dependencies")
        });

        assert_eq!(processed.libraries.len(), 1);
        let lib = &processed.libraries[0];
        assert_eq!(lib.name, "explicit_override_lib_ffi");
        assert_eq!(lib.functions.len(), 1);
        assert_eq!(lib.functions[0].name, "forty_two");
        assert_eq!(lib.functions[0].return_type, "u64");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn process_dependencies_e2e_rejects_empty_rust_symbol_override() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "clorus_rustffi_e2e_reject_empty_rust_symbol_{}",
            unique
        ));
        let dep_dir = root.join("invalid-override-lib");
        let iface_dir = root.join("interfaces");
        std::fs::create_dir_all(dep_dir.join("src")).expect("create dep src");
        std::fs::create_dir_all(&iface_dir).expect("create interfaces dir");

        std::fs::write(
            dep_dir.join("Cargo.toml"),
            r#"[package]
name = "invalid-override-lib"
version = "0.1.0"
edition = "2021"
"#,
        )
        .expect("write dep cargo");

        std::fs::write(
            dep_dir.join("src/lib.rs"),
            r#"
pub fn answer() -> u64 { 42 }
"#,
        )
        .expect("write dep lib");

        std::fs::write(
            iface_dir.join("invalid-override-lib.clri"),
            r#"(interface invalid-override-lib
  (fn forty-two [] :u64 :rust "   ")
)"#,
        )
        .expect("write interface");

        let manifest: Manifest = toml::from_str(
            r#"[package]
name = "demo"
version = "0.1.0"

[build]
entry = "src/main.clrs"

[rust-dependencies]
invalid-override-lib = { path = "invalid-override-lib", interface = "interfaces/invalid-override-lib.clri" }
"#,
        )
        .expect("parse manifest");

        let result = with_cwd(&root, || RustFfiProcessor::process_dependencies(&manifest, false));
        let err = match result {
            Ok(_) => panic!("expected invalid rust symbol override error"),
            Err(e) => e,
        };
        assert!(err.contains("Invalid :rust override for function 'forty-two'"));
        assert!(err.contains("interfaces/invalid-override-lib.clri"));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn process_dependencies_e2e_rejects_duplicate_normalized_interface_names() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "clorus_rustffi_e2e_reject_dup_iface_names_{}",
            unique
        ));
        let dep_dir = root.join("dup-iface-lib");
        let iface_dir = root.join("interfaces");
        std::fs::create_dir_all(dep_dir.join("src")).expect("create dep src");
        std::fs::create_dir_all(&iface_dir).expect("create interfaces dir");

        std::fs::write(
            dep_dir.join("Cargo.toml"),
            r#"[package]
name = "dup-iface-lib"
version = "0.1.0"
edition = "2021"
"#,
        )
        .expect("write dep cargo");

        std::fs::write(
            dep_dir.join("src/lib.rs"),
            r#"
pub fn foo_bar() -> u64 { 1 }
pub fn foo_bar_alt() -> u64 { 2 }
"#,
        )
        .expect("write dep lib");

        std::fs::write(
            iface_dir.join("dup-iface-lib.clri"),
            r#"(interface dup-iface-lib
  (fn foo-bar [] :u64 :rust "foo_bar")
  (fn foo_bar [] :u64 :rust "foo_bar_alt")
)"#,
        )
        .expect("write interface");

        let manifest: Manifest = toml::from_str(
            r#"[package]
name = "demo"
version = "0.1.0"

[build]
entry = "src/main.clrs"

[rust-dependencies]
dup-iface-lib = { path = "dup-iface-lib", interface = "interfaces/dup-iface-lib.clri" }
"#,
        )
        .expect("parse manifest");

        let result = with_cwd(&root, || RustFfiProcessor::process_dependencies(&manifest, false));
        let err = match result {
            Ok(_) => panic!("expected duplicate normalized function-name error"),
            Err(e) => e,
        };
        assert!(err.contains(
            "defines duplicate exported function name 'foo_bar' after normalization"
        ));
        assert!(err.contains("interfaces/dup-iface-lib.clri"));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn process_dependencies_e2e_rejects_invalid_normalized_export_symbol() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "clorus_rustffi_e2e_reject_invalid_export_symbol_{}",
            unique
        ));
        let dep_dir = root.join("invalid-export-lib");
        let iface_dir = root.join("interfaces");
        std::fs::create_dir_all(dep_dir.join("src")).expect("create dep src");
        std::fs::create_dir_all(&iface_dir).expect("create interfaces dir");

        std::fs::write(
            dep_dir.join("Cargo.toml"),
            r#"[package]
name = "invalid-export-lib"
version = "0.1.0"
edition = "2021"
"#,
        )
        .expect("write dep cargo");

        std::fs::write(
            dep_dir.join("src/lib.rs"),
            r#"
pub fn answer() -> u64 { 42 }
"#,
        )
        .expect("write dep lib");

        std::fs::write(
            iface_dir.join("invalid-export-lib.clri"),
            r#"(interface invalid-export-lib
  (fn answer? [] :u64 :rust "answer")
)"#,
        )
        .expect("write interface");

        let manifest: Manifest = toml::from_str(
            r#"[package]
name = "demo"
version = "0.1.0"

[build]
entry = "src/main.clrs"

[rust-dependencies]
invalid-export-lib = { path = "invalid-export-lib", interface = "interfaces/invalid-export-lib.clri" }
"#,
        )
        .expect("parse manifest");

        let result = with_cwd(&root, || RustFfiProcessor::process_dependencies(&manifest, false));
        let err = match result {
            Ok(_) => panic!("expected invalid normalized export symbol error"),
            Err(e) => e,
        };
        assert!(err.contains("normalizes to invalid export symbol 'answer?'"));
        assert!(err.contains("interfaces/invalid-export-lib.clri"));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn process_dependencies_e2e_rejects_empty_interface_definition() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "clorus_rustffi_e2e_reject_empty_interface_definition_{}",
            unique
        ));
        let dep_dir = root.join("empty-iface-lib");
        let iface_dir = root.join("interfaces");
        std::fs::create_dir_all(dep_dir.join("src")).expect("create dep src");
        std::fs::create_dir_all(&iface_dir).expect("create interfaces dir");

        std::fs::write(
            dep_dir.join("Cargo.toml"),
            r#"[package]
name = "empty-iface-lib"
version = "0.1.0"
edition = "2021"
"#,
        )
        .expect("write dep cargo");

        std::fs::write(
            dep_dir.join("src/lib.rs"),
            r#"
pub fn answer() -> u64 { 42 }
"#,
        )
        .expect("write dep lib");

        std::fs::write(
            iface_dir.join("empty-iface-lib.clri"),
            r#"(interface empty-iface-lib)"#,
        )
        .expect("write interface");

        let manifest: Manifest = toml::from_str(
            r#"[package]
name = "demo"
version = "0.1.0"

[build]
entry = "src/main.clrs"

[rust-dependencies]
empty-iface-lib = { path = "empty-iface-lib", interface = "interfaces/empty-iface-lib.clri" }
"#,
        )
        .expect("parse manifest");

        let result = with_cwd(&root, || RustFfiProcessor::process_dependencies(&manifest, false));
        let err = match result {
            Ok(_) => panic!("expected empty interface rejection"),
            Err(e) => e,
        };
        assert!(err.contains("contains no function definitions"));
        assert!(err.contains("interfaces/empty-iface-lib.clri"));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn process_dependencies_e2e_rejects_invalid_rust_symbol_override_path() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "clorus_rustffi_e2e_reject_invalid_rust_symbol_path_{}",
            unique
        ));
        let dep_dir = root.join("invalid-rust-symbol-lib");
        let iface_dir = root.join("interfaces");
        std::fs::create_dir_all(dep_dir.join("src")).expect("create dep src");
        std::fs::create_dir_all(&iface_dir).expect("create interfaces dir");

        std::fs::write(
            dep_dir.join("Cargo.toml"),
            r#"[package]
name = "invalid-rust-symbol-lib"
version = "0.1.0"
edition = "2021"
"#,
        )
        .expect("write dep cargo");

        std::fs::write(
            dep_dir.join("src/lib.rs"),
            r#"
pub struct Math;
impl Math {
    pub fn answer() -> u64 { 42 }
}
"#,
        )
        .expect("write dep lib");

        std::fs::write(
            iface_dir.join("invalid-rust-symbol-lib.clri"),
            r#"(interface invalid-rust-symbol-lib
  (fn forty-two [] :u64 :rust "Math::")
)"#,
        )
        .expect("write interface");

        let manifest: Manifest = toml::from_str(
            r#"[package]
name = "demo"
version = "0.1.0"

[build]
entry = "src/main.clrs"

[rust-dependencies]
invalid-rust-symbol-lib = { path = "invalid-rust-symbol-lib", interface = "interfaces/invalid-rust-symbol-lib.clri" }
"#,
        )
        .expect("parse manifest");

        let result = with_cwd(&root, || RustFfiProcessor::process_dependencies(&manifest, false));
        let err = match result {
            Ok(_) => panic!("expected invalid rust symbol override path rejection"),
            Err(e) => e,
        };
        assert!(err.contains("is not a valid Rust path symbol"));
        assert!(err.contains("Math::"));
        assert!(err.contains("interfaces/invalid-rust-symbol-lib.clri"));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn process_dependencies_e2e_rejects_unsupported_interface_types() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "clorus_rustffi_e2e_reject_unsupported_interface_types_{}",
            unique
        ));
        let dep_dir = root.join("unsupported-types-lib");
        let iface_dir = root.join("interfaces");
        std::fs::create_dir_all(dep_dir.join("src")).expect("create dep src");
        std::fs::create_dir_all(&iface_dir).expect("create interfaces dir");

        std::fs::write(
            dep_dir.join("Cargo.toml"),
            r#"[package]
name = "unsupported-types-lib"
version = "0.1.0"
edition = "2021"
"#,
        )
        .expect("write dep cargo");

        std::fs::write(
            dep_dir.join("src/lib.rs"),
            r#"
pub fn bytes_len(v: Vec<u8>) -> usize { v.len() }
"#,
        )
        .expect("write dep lib");

        std::fs::write(
            iface_dir.join("unsupported-types-lib.clri"),
            r#"(interface unsupported-types-lib
  (fn bytes-len [v :Vec<u8>] :usize :rust "bytes_len")
)"#,
        )
        .expect("write interface");

        let manifest: Manifest = toml::from_str(
            r#"[package]
name = "demo"
version = "0.1.0"

[build]
entry = "src/main.clrs"

[rust-dependencies]
unsupported-types-lib = { path = "unsupported-types-lib", interface = "interfaces/unsupported-types-lib.clri" }
"#,
        )
        .expect("parse manifest");

        let result = with_cwd(&root, || RustFfiProcessor::process_dependencies(&manifest, false));
        let err = match result {
            Ok(_) => panic!("expected unsupported interface type rejection"),
            Err(e) => e,
        };
        assert!(err.contains("unsupported param type 'Vec<u8>'"));
        assert!(err.contains("interfaces/unsupported-types-lib.clri"));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn process_dependencies_e2e_rejects_unsupported_interface_return_type() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "clorus_rustffi_e2e_reject_unsupported_interface_return_type_{}",
            unique
        ));
        let dep_dir = root.join("unsupported-return-lib");
        let iface_dir = root.join("interfaces");
        std::fs::create_dir_all(dep_dir.join("src")).expect("create dep src");
        std::fs::create_dir_all(&iface_dir).expect("create interfaces dir");

        std::fs::write(
            dep_dir.join("Cargo.toml"),
            r#"[package]
name = "unsupported-return-lib"
version = "0.1.0"
edition = "2021"
"#,
        )
        .expect("write dep cargo");

        std::fs::write(
            dep_dir.join("src/lib.rs"),
            r#"
pub fn make_bytes() -> Vec<u8> { vec![1, 2, 3] }
"#,
        )
        .expect("write dep lib");

        std::fs::write(
            iface_dir.join("unsupported-return-lib.clri"),
            r#"(interface unsupported-return-lib
  (fn make-bytes [] :Vec<u8> :rust "make_bytes")
)"#,
        )
        .expect("write interface");

        let manifest: Manifest = toml::from_str(
            r#"[package]
name = "demo"
version = "0.1.0"

[build]
entry = "src/main.clrs"

[rust-dependencies]
unsupported-return-lib = { path = "unsupported-return-lib", interface = "interfaces/unsupported-return-lib.clri" }
"#,
        )
        .expect("parse manifest");

        let result = with_cwd(&root, || RustFfiProcessor::process_dependencies(&manifest, false));
        let err = match result {
            Ok(_) => panic!("expected unsupported interface return type rejection"),
            Err(e) => e,
        };
        assert!(err.contains("unsupported return type 'Vec<u8>'"));
        assert!(err.contains("interfaces/unsupported-return-lib.clri"));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn process_dependencies_e2e_rejects_unsupported_interface_pointer_type() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "clorus_rustffi_e2e_reject_unsupported_interface_pointer_type_{}",
            unique
        ));
        let dep_dir = root.join("unsupported-pointer-lib");
        let iface_dir = root.join("interfaces");
        std::fs::create_dir_all(dep_dir.join("src")).expect("create dep src");
        std::fs::create_dir_all(&iface_dir).expect("create interfaces dir");

        std::fs::write(
            dep_dir.join("Cargo.toml"),
            r#"[package]
name = "unsupported-pointer-lib"
version = "0.1.0"
edition = "2021"
"#,
        )
        .expect("write dep cargo");

        std::fs::write(
            dep_dir.join("src/lib.rs"),
            r#"
pub fn ptr_roundtrip(p: *mut i32) -> *mut i32 { p }
"#,
        )
        .expect("write dep lib");

        std::fs::write(
            iface_dir.join("unsupported-pointer-lib.clri"),
            r#"(interface unsupported-pointer-lib
  (fn ptr-roundtrip [p :ptr-i32] :ptr-i32)
)"#,
        )
        .expect("write interface");

        let manifest: Manifest = toml::from_str(
            r#"[package]
name = "demo"
version = "0.1.0"

[build]
entry = "src/main.clrs"

[rust-dependencies]
unsupported-pointer-lib = { path = "unsupported-pointer-lib", interface = "interfaces/unsupported-pointer-lib.clri" }
"#,
        )
        .expect("parse manifest");

        let err = with_cwd(&root, || match RustFfiProcessor::process_dependencies(&manifest, false) {
            Ok(_) => panic!("expected unsupported interface pointer-type rejection"),
            Err(e) => e,
        });
        assert!(
            err.contains("unsupported param type"),
            "expected unsupported param-type rejection, got: {}",
            err
        );
        assert!(
            err.contains("ptr-i32"),
            "expected unsupported pointer-ish type context, got: {}",
            err
        );
        assert!(
            err.contains("unsupported-pointer-lib.clri"),
            "expected interface path context, got: {}",
            err
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn process_dependencies_e2e_reports_interface_context_on_wrapper_compile_failure() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "clorus_rustffi_e2e_interface_compile_context_{}",
            unique
        ));
        let dep_dir = root.join("bad-rust-symbol-lib");
        let iface_dir = root.join("interfaces");
        std::fs::create_dir_all(dep_dir.join("src")).expect("create dep src");
        std::fs::create_dir_all(&iface_dir).expect("create interfaces dir");

        std::fs::write(
            dep_dir.join("Cargo.toml"),
            r#"[package]
name = "bad-rust-symbol-lib"
version = "0.1.0"
edition = "2021"
"#,
        )
        .expect("write dep cargo");

        std::fs::write(dep_dir.join("src/lib.rs"), "pub fn ping(x: i32) -> i32 { x }\n")
            .expect("write dep lib");

        // Valid Rust symbol syntax, but unresolved in dependency crate.
        std::fs::write(
            iface_dir.join("bad-rust-symbol-lib.clri"),
            r#"(interface bad-rust-symbol-lib
  (fn ping [x :i32] :i32 :rust "missing_symbol")
)"#,
        )
        .expect("write interface");

        let manifest: Manifest = toml::from_str(
            r#"[package]
name = "demo"
version = "0.1.0"

[build]
entry = "src/main.clrs"

[rust-dependencies]
bad-rust-symbol-lib = { path = "bad-rust-symbol-lib", interface = "interfaces/bad-rust-symbol-lib.clri" }
"#,
        )
        .expect("parse manifest");

        let err = with_cwd(&root, || match RustFfiProcessor::process_dependencies(&manifest, false) {
            Ok(_) => panic!("expected compile failure from unresolved :rust symbol"),
            Err(e) => e,
        });

        assert!(err.contains("Failed while compiling wrapper generated from interface"));
        assert!(err.contains("interfaces/bad-rust-symbol-lib.clri"));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn process_dependencies_e2e_reports_interface_tokenize_context() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "clorus_rustffi_e2e_interface_tokenize_context_{}",
            unique
        ));
        let dep_dir = root.join("bad-token-lib");
        let iface_dir = root.join("interfaces");
        std::fs::create_dir_all(dep_dir.join("src")).expect("create dep src");
        std::fs::create_dir_all(&iface_dir).expect("create interfaces dir");

        std::fs::write(
            dep_dir.join("Cargo.toml"),
            r#"[package]
name = "bad-token-lib"
version = "0.1.0"
edition = "2021"
"#,
        )
        .expect("write dep cargo");

        std::fs::write(dep_dir.join("src/lib.rs"), "pub fn ping() -> i32 { 1 }\n")
            .expect("write dep lib");

        // Deliberately malformed tokenizer input: unterminated string.
        std::fs::write(
            iface_dir.join("bad-token-lib.clri"),
            r#"(interface bad-token-lib
  (fn ping [] :i32 :rust "unterminated)
)"#,
        )
        .expect("write malformed interface");

        let manifest: Manifest = toml::from_str(
            r#"[package]
name = "demo"
version = "0.1.0"

[build]
entry = "src/main.clrs"

[rust-dependencies]
bad-token-lib = { path = "bad-token-lib", interface = "interfaces/bad-token-lib.clri" }
"#,
        )
        .expect("parse manifest");

        let err = with_cwd(&root, || match RustFfiProcessor::process_dependencies(&manifest, false) {
            Ok(_) => panic!("expected tokenize failure"),
            Err(e) => e,
        });

        assert!(err.contains("Failed to tokenize interface file"));
        assert!(err.contains("interfaces/bad-token-lib.clri"));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn process_dependencies_reports_dependency_name_for_missing_path() {
        let manifest: Manifest = toml::from_str(
            r#"[package]
name = "demo"
version = "0.1.0"

[build]
entry = "src/main.clrs"

[rust-dependencies]
missing-lib = { path = "definitely-not-here-lib" }
"#,
        )
        .expect("parse manifest");

        let err = match RustFfiProcessor::process_dependencies(&manifest, false) {
            Ok(_) => panic!("expected missing dependency path failure"),
            Err(e) => e,
        };
        assert!(err.contains("Rust dependency 'missing-lib' path not found"));
        assert!(err.contains("definitely-not-here-lib"));
    }

    #[test]
    fn process_dependencies_e2e_reports_interface_parse_location_context() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "clorus_rustffi_e2e_interface_parse_location_{}",
            unique
        ));
        let dep_dir = root.join("bad-iface-lib");
        let iface_dir = root.join("interfaces");
        std::fs::create_dir_all(dep_dir.join("src")).expect("create dep src");
        std::fs::create_dir_all(&iface_dir).expect("create interfaces dir");

        std::fs::write(
            dep_dir.join("Cargo.toml"),
            r#"[package]
name = "bad-iface-lib"
version = "0.1.0"
edition = "2021"
"#,
        )
        .expect("write dep cargo");

        std::fs::write(dep_dir.join("src/lib.rs"), "pub fn ping(x: i32) -> i32 { x }\n")
            .expect("write dep lib");

        // Deliberately malformed: param type must be keyword (`:i32`), not symbol (`i32`).
        std::fs::write(
            iface_dir.join("bad-iface-lib.clri"),
            r#"(interface bad-iface-lib
  (fn ping [x i32] :i32)
)"#,
        )
        .expect("write malformed interface");

        let manifest: Manifest = toml::from_str(
            r#"[package]
name = "demo"
version = "0.1.0"

[build]
entry = "src/main.clrs"

[rust-dependencies]
bad-iface-lib = { path = "bad-iface-lib", interface = "interfaces/bad-iface-lib.clri" }
"#,
        )
        .expect("parse manifest");

        let err = with_cwd(&root, || match RustFfiProcessor::process_dependencies(&manifest, false) {
            Ok(_) => panic!("expected malformed interface parse failure"),
            Err(e) => e,
        });

        assert!(err.contains("Failed to parse interface file"));
        assert!(err.contains("interfaces/bad-iface-lib.clri"));
        assert!(err.contains("Expected keyword"));
        assert!(err.contains("line"));
        assert!(err.contains("column"));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn process_dependencies_e2e_local_path_auto_interface_uses_legacy_when_clri_absent() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("clorus_rustffi_e2e_auto_legacy_{}", unique));
        let dep_dir = root.join("auto-legacy-lib");
        let iface_dir = root.join("interfaces");
        std::fs::create_dir_all(dep_dir.join("src")).expect("create dep src");
        std::fs::create_dir_all(&iface_dir).expect("create interfaces dir");

        std::fs::write(
            dep_dir.join("Cargo.toml"),
            r#"[package]
name = "auto-legacy-lib"
version = "0.1.0"
edition = "2021"
"#,
        )
        .expect("write dep cargo");

        std::fs::write(dep_dir.join("src/lib.rs"), "pub fn pong() -> i32 { 9 }\n")
            .expect("write dep lib");

        // Intentionally only legacy interface file present.
        std::fs::write(
            iface_dir.join("auto-legacy-lib.clorus-ffi"),
            r#"(interface auto-legacy-lib
  (fn pong [] :i32)
)"#,
        )
        .expect("write legacy interface");

        let manifest: Manifest = toml::from_str(
            r#"[package]
name = "demo"
version = "0.1.0"

[build]
entry = "src/main.clrs"

[rust-dependencies]
auto-legacy-lib = { path = "auto-legacy-lib", interface = true }
"#,
        )
        .expect("parse manifest");

        let processed = with_cwd(&root, || {
            RustFfiProcessor::process_dependencies(&manifest, false)
                .expect("process dependencies")
        });

        assert_eq!(processed.libraries.len(), 1);
        let lib = &processed.libraries[0];
        assert_eq!(lib.name, "auto_legacy_lib_ffi");
        let names: Vec<&str> = lib.functions.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["pong"]);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn process_dependencies_e2e_local_path_auto_interface_prefers_clri_over_legacy() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("clorus_rustffi_e2e_auto_prefers_clri_{}", unique));
        let dep_dir = root.join("prefer-clri-lib");
        let iface_dir = root.join("interfaces");
        std::fs::create_dir_all(dep_dir.join("src")).expect("create dep src");
        std::fs::create_dir_all(&iface_dir).expect("create interfaces dir");

        std::fs::write(
            dep_dir.join("Cargo.toml"),
            r#"[package]
name = "prefer-clri-lib"
version = "0.1.0"
edition = "2021"
"#,
        )
        .expect("write dep cargo");

        std::fs::write(
            dep_dir.join("src/lib.rs"),
            r#"
pub fn ping() -> i32 { 11 }
pub fn pong() -> i32 { 12 }
"#,
        )
        .expect("write dep lib");

        // Both interfaces are present; auto-resolution should prefer .clri.
        std::fs::write(
            iface_dir.join("prefer-clri-lib.clri"),
            r#"(interface prefer-clri-lib
  (fn ping [] :i32)
)"#,
        )
        .expect("write clri interface");
        std::fs::write(
            iface_dir.join("prefer-clri-lib.clorus-ffi"),
            r#"(interface prefer-clri-lib
  (fn pong [] :i32)
)"#,
        )
        .expect("write legacy interface");

        let manifest: Manifest = toml::from_str(
            r#"[package]
name = "demo"
version = "0.1.0"

[build]
entry = "src/main.clrs"

[rust-dependencies]
prefer-clri-lib = { path = "prefer-clri-lib", interface = true }
"#,
        )
        .expect("parse manifest");

        let processed = with_cwd(&root, || {
            RustFfiProcessor::process_dependencies(&manifest, false)
                .expect("process dependencies")
        });

        assert_eq!(processed.libraries.len(), 1);
        let lib = &processed.libraries[0];
        assert_eq!(lib.name, "prefer_clri_lib_ffi");
        let names: Vec<&str> = lib.functions.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["ping"]);

        let _ = std::fs::remove_dir_all(&root);
    }
}
