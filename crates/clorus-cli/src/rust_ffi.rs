/// Automatic Rust FFI processing - Phase 2 Architecture
/// Generates wrapper crates in target/rust-ffi/ instead of polluting library source
use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs;
use clorus_ffi_gen::{FfiGenerator, FunctionInfo};
use crate::manifest::Manifest;
use serde::Deserialize;

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

impl RustFfiProcessor {
    fn should_build_offline(source: &RustDepSource) -> bool {
        matches!(source, RustDepSource::Path(_))
    }

    fn is_supported_ffi_type(type_name: &str) -> bool {
        matches!(type_name, "f64" | "i32" | "bool" | "String" | "()" | "*mut u8")
    }

    fn retain_supported_ffi_functions(
        functions: Vec<FunctionInfo>,
        verbose: bool,
    ) -> Vec<FunctionInfo> {
        let total = functions.len();
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
            let skipped = total.saturating_sub(filtered.len());
            if skipped > 0 {
                println!(
                    "      Skipped {} unsupported function(s) (non-FFI-compatible signature)",
                    skipped
                );
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
        generator.functions = Self::retain_supported_ffi_functions(generator.functions, verbose);

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
        let mut processed = Self::compile_wrapper_crate(
            &wrapper_dir,
            &wrapper_name,
            verbose,
            true,
        )?;
        processed.functions = generator.functions.clone();

        if verbose {
            println!("      Populated with {} function metadata entries", processed.functions.len());
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

        let mut functions = Self::discover_registry_functions(&wrapper_dir, name, verbose)?;
        functions = Self::retain_supported_ffi_functions(functions, verbose);
        if functions.is_empty() {
            return Err(format!(
                "Rust dependency '{}' resolved from registry but no FFI-compatible functions were discovered in its lib target. Add an interface file (interfaces/{}.clri) or use a local bridge crate.",
                name, name
            ));
        }

        if verbose {
            println!("      Discovered {} public function(s)", functions.len());
        }

        let mut generator = FfiGenerator::new();
        generator.functions = functions.clone();
        Self::generate_wrapper_lib_rs(&wrapper_dir, name, &generator)?;

        let mut processed = Self::compile_wrapper_crate(
            &wrapper_dir,
            &wrapper_name,
            verbose,
            false,
        )?;
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
            .find(|p| p.name == dep_name && p.source.as_deref().unwrap_or("").starts_with("registry+"))
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
        let mut processed = Self::compile_wrapper_crate(
            &wrapper_dir,
            &wrapper_name,
            verbose,
            Self::should_build_offline(source),
        )?;
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

    #[test]
    fn path_sources_build_offline_registry_sources_build_online() {
        assert!(RustFfiProcessor::should_build_offline(&RustDepSource::Path(PathBuf::from("."))));
        assert!(!RustFfiProcessor::should_build_offline(&RustDepSource::Version("1.0".to_string())));
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
                    source: Some("registry+https://github.com/rust-lang/crates.io-index".to_string()),
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
    fn retain_supported_ffi_functions_filters_unsupported_signatures() {
        let functions = vec![
            FunctionInfo {
                name: "ok_add".to_string(),
                params: vec![
                    clorus_ffi_gen::ParamInfo { name: "a".to_string(), type_name: "f64".to_string() },
                    clorus_ffi_gen::ParamInfo { name: "b".to_string(), type_name: "f64".to_string() },
                ],
                return_type: "f64".to_string(),
            },
            FunctionInfo {
                name: "bad_u64".to_string(),
                params: vec![clorus_ffi_gen::ParamInfo {
                    name: "n".to_string(),
                    type_name: "u64".to_string(),
                }],
                return_type: "u64".to_string(),
            },
        ];

        let filtered = RustFfiProcessor::retain_supported_ffi_functions(functions, false);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "ok_add");
    }
}
