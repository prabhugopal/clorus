/// Automatic Rust FFI processing - Phase 2 Architecture
/// Generates wrapper crates in target/rust-ffi/ instead of polluting library source
use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs;
use clorus_ffi_gen::{FfiGenerator, FunctionInfo};
use crate::manifest::Manifest;

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

impl RustFfiProcessor {
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
            println!("   Processing {} Rust dependencies...", manifest.rust_dependencies.len());
        }

        for (name, dep) in &manifest.rust_dependencies {
            if verbose {
                println!("   → {}", name);
            }

            let source = if let Some(path) = dep.get_path() {
                let lib_path = PathBuf::from(path);
                if !lib_path.exists() {
                    return Err(format!(
                        "Rust dependency path not found: {}",
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
                let interface_path = match interface_spec {
                    crate::manifest::InterfaceSpec::Path(path) => path,
                    crate::manifest::InterfaceSpec::Auto => {
                        // Auto-discover: Try .clri first (preferred), fall back to .clorus-ffi (legacy)
                        let clri_path = format!("interfaces/{}.clri", name);
                        let legacy_path = format!("interfaces/{}.clorus-ffi", name);

                        if std::path::Path::new(&clri_path).exists() {
                            clri_path
                        } else {
                            legacy_path
                        }
                    }
                };

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
                        return Err(format!(
                            "Rust dependency '{}' uses version '{}' but has no interface. Add interface = \"interfaces/{}.clri\" for registry dependencies.",
                            name, version, name
                        ));
                    }
                }
            };

            processor.libraries.push(processed);
        }

        Ok(processor)
    }

    /// Create wrapper crate in target/rust-ffi/ and compile it
    fn create_and_compile_wrapper(name: &str, lib_path: &Path, verbose: bool) -> Result<ProcessedLibrary, String> {
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
            return Err(format!("Rust library source not found: {}", lib_src.display()));
        }

        let mut generator = FfiGenerator::new();
        generator.parse_file(&lib_src)?;

        if generator.functions.is_empty() {
            if verbose {
                println!("      No public functions found - creating empty wrapper");
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
        let mut processed = Self::compile_wrapper_crate(&wrapper_dir, &wrapper_name, verbose)?;
        processed.functions = generator.functions.clone();

        if verbose {
            println!("      Populated with {} function metadata entries", processed.functions.len());
        }

        Ok(processed)
    }

    /// Generate Cargo.toml for the wrapper crate
    fn generate_wrapper_cargo_toml(
        wrapper_dir: &Path,
        wrapper_name: &str,
        original_name: &str,
        original_path: &Path,
    ) -> Result<(), String> {
        // Convert original path to absolute
        let original_abs = original_path.canonicalize()
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
            original_name,  // Keep package name with hyphens
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
                let original_abs = original_path.canonicalize()
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
            wrapper_name,
            original_name,
            dependency_spec
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

    /// Compile the wrapper crate
    fn compile_wrapper_crate(
        wrapper_dir: &Path,
        wrapper_name: &str,
        verbose: bool,
    ) -> Result<ProcessedLibrary, String> {
        if verbose {
            println!("      Compiling wrapper crate...");
        }

        // Run cargo build --release --offline (use cached dependencies)
        let output = Command::new("cargo")
            .arg("build")
            .arg("--release")
            .arg("--offline")
            .current_dir(wrapper_dir)
            .output()
            .map_err(|e| format!("Failed to run cargo build on wrapper: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Wrapper crate build failed:\n{}", stderr));
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
        verbose: bool
    ) -> Result<ProcessedLibrary, String> {
        // Parse interface file
        let interface_file_path = PathBuf::from(interface_path);
        if !interface_file_path.exists() {
            return Err(format!(
                "Interface file not found: {}",
                interface_file_path.display()
            ));
        }

        use crate::interface::parse_interface_file;
        let interface = parse_interface_file(&interface_file_path)?;

        if verbose {
            println!("      Parsed interface: {} functions", interface.functions.len());
        }

        // Convert interface functions to FunctionInfo
        let functions: Vec<FunctionInfo> = interface.functions.iter().map(|f| {
            // Convert kebab-case to snake_case for Rust compatibility
            let rust_name = f.name.replace('-', "_");

            FunctionInfo {
                name: rust_name,
                params: f.params.iter().map(|p| {
                    clorus_ffi_gen::ParamInfo {
                        name: p.name.clone(),
                        type_name: p.type_name.clone(),
                    }
                }).collect(),
                return_type: f.return_type.clone(),
            }
        }).collect();

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

        // Generate lib.rs using interface functions
        let mut generator = FfiGenerator::new();
        generator.functions = functions.clone();
        Self::generate_wrapper_lib_rs(&wrapper_dir, name, &generator)?;

        if verbose {
            println!("      Generated wrapper from interface");
        }

        // Compile and return
        let mut processed = Self::compile_wrapper_crate(&wrapper_dir, &wrapper_name, verbose)?;
        processed.functions = functions;

        Ok(processed)
    }

    /// Get paths to all static libraries for linking
    pub fn get_static_lib_paths(&self) -> Vec<PathBuf> {
        self.libraries.iter()
            .map(|lib| lib.static_lib_path.clone())
            .collect()
    }

    /// Get paths to all dynamic libraries for JIT loading
    pub fn get_dynamic_lib_paths(&self) -> Vec<PathBuf> {
        self.libraries.iter()
            .filter_map(|lib| lib.dynamic_lib_path.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

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
        assert!(result.is_ok(), "failed to generate cargo toml: {:?}", result.err());

        let cargo_toml =
            std::fs::read_to_string(wrapper_dir.join("Cargo.toml")).expect("read generated cargo");
        assert!(cargo_toml.contains("demo-math = \"0.1.0\""));

        let _ = std::fs::remove_dir_all(wrapper_dir);
    }
}
