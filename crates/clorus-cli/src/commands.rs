/// Commands for the Clorus CLI tool
use std::fs;
use std::path::{Path, PathBuf};
use std::collections::{HashSet, HashMap};
use std::io::{Write, Read};
use crate::manifest::Manifest;
use clorus_syntax::Expr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewTemplate {
    Basic,
    RustInterop,
}

pub fn new(name: &str) -> Result<(), String> {
    new_with_template(name, NewTemplate::Basic)
}

pub fn new_with_template(name: &str, template: NewTemplate) -> Result<(), String> {
    let project_path = Path::new(name);

    if project_path.exists() {
        return Err(format!("Directory '{}' already exists", name));
    }

    // Create project structure
    fs::create_dir_all(project_path.join("src"))
        .map_err(|e| format!("Failed to create directories: {}", e))?;
    if template == NewTemplate::RustInterop {
        fs::create_dir_all(project_path.join("interfaces"))
            .map_err(|e| format!("Failed to create interfaces directory: {}", e))?;
    }

    // Create Clorus.toml
    let manifest_content = match template {
        NewTemplate::Basic => format!(
            r#"[package]
name = "{}"
version = "0.1.0"
authors = []

[build]
entry = "src/main.clrs"
"#,
            name
        ),
        NewTemplate::RustInterop => format!(
            r#"[package]
name = "{}"
version = "0.1.0"
authors = []

[build]
entry = "src/main.clrs"

[rust-dependencies]
libm = {{ version = "0.2", interface = "interfaces/libm.clri" }}
"#,
            name
        ),
    };

    fs::write(project_path.join("Clorus.toml"), manifest_content)
        .map_err(|e| format!("Failed to create Clorus.toml: {}", e))?;

    // Create src/main.clrs with modern namespace + -main entrypoint template.
    let main_content = match template {
        NewTemplate::Basic => format!(
            r#"; {} entrypoint
(ns main)

(defn -main [& _args]
  (do
    (println "Hello from {}!")
    0))
"#,
            name, name
        ),
        NewTemplate::RustInterop => r#"; Rust interop starter template
; Canonical import style: ns :rust clause.
(ns main
  (:rust [libm :as m]))

(defn -main [& _args]
  (do
    (println "Rust interop starter")
    (println "sin(0.0) =" (m/sin 0.0))
    (println "cos(0.0) =" (m/cos 0.0))
    0))
"#
        .to_string(),
    };

    fs::write(project_path.join("src/main.clrs"), main_content)
        .map_err(|e| format!("Failed to create main.clrs: {}", e))?;

    if template == NewTemplate::RustInterop {
        let interface_content = r#"(interface libm
  (fn sin [x :f64] :f64)
  (fn cos [x :f64] :f64))
"#;
        fs::write(
            project_path.join("interfaces").join("libm.clri"),
            interface_content,
        )
        .map_err(|e| format!("Failed to create interfaces/libm.clri: {}", e))?;
    }

    // Create .gitignore
    let gitignore_content = r#"target/
*.clrs.tmp
"#;

    fs::write(project_path.join(".gitignore"), gitignore_content)
        .map_err(|e| format!("Failed to create .gitignore: {}", e))?;

    println!("     Created binary (application) `{}` package", name);
    println!();
    println!("To get started:");
    println!("  cd {}", name);
    println!("  clorus run");

    Ok(())
}

/// Module loader for :require statements
/// Recursively loads all required modules and returns expressions in dependency order
fn load_module_recursive(
    namespace: &str,
    loaded_modules: &mut HashSet<String>,
    base_path: &Path,
    clip_namespaces: &HashSet<String>,
    debug: bool,
) -> Result<Vec<Expr>, String> {
    // Skip if already loaded
    if loaded_modules.contains(namespace) {
        return Ok(Vec::new());
    }

    // Mark as loaded (before recursion to handle circular deps)
    loaded_modules.insert(namespace.to_string());

    // Convert namespace to file path
    // coral.widgets → src/coral/widgets.clrs
    let file_path = namespace_to_path(namespace, base_path)?;

    if debug {
        println!("     → Found: {}", file_path.display());
    }

    // Read the file
    let source = fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read module {}: {}", namespace, e))?;

    // Parse (with macro expansion)
    let exprs = clorus::parse_and_expand(&source)
        .map_err(|e| format!("Parse error in {}: {}", file_path.display(), e))?;

    // Find all :require statements and recursively load dependencies
    let mut all_exprs = Vec::new();

    for expr in &exprs {
        // Check top-level Expr::Require
        if let Expr::Require { specs } = expr {
            for spec in specs {
                // Skip if from .clip package
                let is_clip = clip_namespaces.iter().any(|prefix| {
                    spec.module.starts_with(prefix)
                });
                if is_clip {
                    continue;
                }
                let dep_exprs = load_module_recursive(&spec.module, loaded_modules, base_path, clip_namespaces, debug)?;
                all_exprs.extend(dep_exprs);
            }
        }

        // Check Expr::Ns for embedded :require clauses
        if let Expr::Ns { requires, .. } = expr {
            for spec in requires {
                // Skip if from .clip package
                let is_clip = clip_namespaces.iter().any(|prefix| {
                    spec.module.starts_with(prefix)
                });
                if is_clip {
                    continue;
                }
                let dep_exprs = load_module_recursive(&spec.module, loaded_modules, base_path, clip_namespaces, debug)?;
                all_exprs.extend(dep_exprs);
            }
        }
    }

    // Add this module's expressions AFTER its dependencies
    all_exprs.extend(exprs);

    Ok(all_exprs)
}

/// Convert namespace to file path
/// coral.widgets → src/coral/widgets.clrs
/// coral.core → src/coral/core.clrs
/// clorus.core → $CLORUS_HOME/stdlib/clorus/core.clr or ~/.clorus/stdlib/clorus/core.clr (global stdlib)
fn namespace_to_path(namespace: &str, base_path: &Path) -> Result<PathBuf, String> {
    let parts: Vec<&str> = namespace.split('.').collect();

    if parts.is_empty() {
        return Err(format!("Invalid namespace: {}", namespace));
    }

    // Check if this is a clorus.* namespace (stdlib)
    if parts[0] == "clorus" {
        // Clojure-style mapping: clorus.set -> stdlib/clorus/set.clr
        // Search order:
        // 1) CLORUS_HOME/stdlib
        // 2) ~/.clorus/stdlib
        // 3) cwd/stdlib (repo/project local)
        // 4) paths relative to current executable (installed/repo layouts)
        let mut stdlib_paths = vec![
            std::env::var("CLORUS_HOME").ok().map(|home| PathBuf::from(home).join("stdlib")),
            std::env::var("HOME").ok().map(|home| PathBuf::from(home).join(".clorus/stdlib")),
            Some(base_path.join("stdlib")),
        ];

        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                stdlib_paths.push(Some(exe_dir.join("../stdlib"))); // installed layout
                stdlib_paths.push(Some(exe_dir.join("../../stdlib"))); // repo target/{debug,release}
                stdlib_paths.push(Some(exe_dir.join("../../../stdlib"))); // extra fallback
            }
        }

        // Convert clorus.string.utils -> clorus/string/utils.clr
        let mut ns_path = PathBuf::new();
        for part in &parts[..parts.len() - 1] {
            ns_path = ns_path.join(part);
        }
        ns_path = ns_path.join(format!("{}.clr", parts[parts.len() - 1]));

        for stdlib_dir in stdlib_paths.into_iter().flatten() {
            let stdlib_path = stdlib_dir.join(&ns_path);
            if stdlib_path.exists() {
                return Ok(stdlib_path);
            }
        }

        return Err(format!(
            "Stdlib module not found: {} (expected {} in CLORUS_HOME/stdlib or ~/.clorus/stdlib)",
            namespace,
            ns_path.to_string_lossy()
        ));
    }

    // Regular project module: src/coral/widgets.clrs
    let mut path = base_path.join("src");

    // Add directory components
    for part in &parts[..parts.len()-1] {
        path = path.join(part);
    }

    // Add file name
    let file_name = format!("{}.clrs", parts[parts.len()-1]);
    path = path.join(file_name);

    if !path.exists() {
        return Err(format!("Module file not found: {} (looking for {})", namespace, path.display()));
    }

    Ok(path)
}

pub fn check() -> Result<(), String> {
    let manifest = Manifest::find_in_current_dir()?;

    // Determine entry point: explicit entry, src/lib.clrs, lib.clrs, src/lib.clr, lib.clr, or skip
    let entry = match &manifest.build.entry {
        Some(e) => e,
        None => {
            // No explicit entry - check for library entry points
            if Path::new("src/lib.clrs").exists() {
                "src/lib.clrs"
            } else if Path::new("lib.clrs").exists() {
                "lib.clrs"
            } else if Path::new("src/lib.clr").exists() {
                "src/lib.clr"
            } else if Path::new("lib.clr").exists() {
                "lib.clr"
            } else {
                // No entry and no lib file - nothing to check
                println!("   Skipping check for library package {} v{}",
                    manifest.package.name, manifest.package.version);
                println!("   (No entry point or src/lib.clrs found)");
                return Ok(());
            }
        }
    };

    let entry_path = Path::new(entry);

    if !entry_path.exists() {
        return Err(format!("Entry file '{}' not found", entry));
    }

    println!("   Checking {} v{}", manifest.package.name, manifest.package.version);

    let source = fs::read_to_string(entry_path)
        .map_err(|e| format!("Failed to read {}: {}", entry, e))?;

    // Parse to check syntax (with macro expansion)
    clorus::parse_and_expand(&source)
        .map_err(|e| format!("Parse error in {}: {}", entry, e))?;

    println!("    Finished checking {} in 0.00s", manifest.package.name);

    Ok(())
}

pub fn clean() -> Result<(), String> {
    let manifest = Manifest::find_in_current_dir()?;

    println!("   Cleaning {} v{}", manifest.package.name, manifest.package.version);

    let mut cleaned_items = Vec::new();

    // Remove target directory
    let target_dir = Path::new("target");
    if target_dir.exists() {
        fs::remove_dir_all(target_dir)
            .map_err(|e| format!("Failed to remove target directory: {}", e))?;
        cleaned_items.push("target/".to_string());
    }

    // Remove .clip files (packaged artifacts)
    if let Ok(entries) = fs::read_dir(".") {
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            if let Some(name) = file_name.to_str() {
                if name.starts_with(&manifest.package.name) && name.ends_with(".clip") {
                    fs::remove_file(entry.path())
                        .map_err(|e| format!("Failed to remove .clip file: {}", e))?;
                    cleaned_items.push(name.to_string());
                }
            }
        }
    }

    if cleaned_items.is_empty() {
        println!("      Nothing to clean");
    } else {
        for item in &cleaned_items {
            println!("      Removed {}", item);
        }
    }

    println!("    Finished cleaning");

    Ok(())
}

pub fn build() -> Result<(), String> {
    build_internal(false, false)
}

pub fn build_with_debug(debug: bool) -> Result<(), String> {
    build_internal(false, debug)
}

pub fn build_lib() -> Result<(), String> {
    build_internal(true, false)
}

fn get_entry_override() -> Option<String> {
    std::env::var("CLORUS_ENTRY_FILE")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn build_internal(mut lib_mode: bool, debug: bool) -> Result<(), String> {
    let manifest = Manifest::find_in_current_dir()?;

    // Determine entry point: explicit entry, src/lib.clrs, lib.clrs, src/lib.clr, lib.clr, or skip
    let entry = if let Some(override_entry) = get_entry_override() {
        // Explicit test/runner override always targets an executable entry.
        lib_mode = false;
        override_entry
    } else {
        match &manifest.build.entry {
            Some(e) => {
                // Check if explicitly specified entry is a library entry point
                if e == "src/lib.clrs" || e == "lib.clrs" || e == "src/lib.clr" || e == "lib.clr" {
                    lib_mode = true;
                }
                e.clone()
            },
            None => {
                // No explicit entry - check for library entry points
                // When using implicit lib.clrs, this is a library package
                lib_mode = true;

                if Path::new("src/lib.clrs").exists() {
                    "src/lib.clrs".to_string()
                } else if Path::new("lib.clrs").exists() {
                    "lib.clrs".to_string()
                } else if Path::new("src/lib.clr").exists() {
                    "src/lib.clr".to_string()
                } else if Path::new("lib.clr").exists() {
                    "lib.clr".to_string()
                } else {
                    // No entry and no lib file - this is a library with no code to compile
                    println!("   Skipping build for library package {} v{}",
                        manifest.package.name, manifest.package.version);
                    println!("   (No entry point or src/lib.clrs found)");
                    return Ok(());
                }
            }
        }
    };

    let entry_path = Path::new(&entry);

    if !entry_path.exists() {
        return Err(format!("Entry file '{}' not found", entry));
    }

    // Process Rust dependencies (auto-generate FFI and compile)
    let rust_ffi = crate::rust_ffi::RustFfiProcessor::process_dependencies(&manifest, true)?;

    // Load .clip dependencies automatically (Phase 3)
    println!("   Resolving dependencies...");
    let clip_packages = crate::pack::load_clip_dependencies(&manifest)?;

    if !clip_packages.is_empty() {
        println!("   Loaded {} .clip package(s)", clip_packages.len());
        for package in &clip_packages {
            println!("      ✓ {} v{}", package.name, package.version);
        }
    }

    println!("   Compiling {} v{}", manifest.package.name, manifest.package.version);

    // Load and parse stdlib/clorus/core.clr and stdlib/clorus/transducers.clr (optional)
    let mut all_exprs = Vec::new();
    if manifest.build.stdlib {
        let stdlib_path = Path::new("stdlib/clorus/core.clr");
        if stdlib_path.exists() {
            let stdlib_source = fs::read_to_string(stdlib_path)
                .map_err(|e| format!("Failed to read stdlib/clorus/core.clr: {}", e))?;

            let stdlib_exprs = clorus::parse_and_expand(&stdlib_source)
                .map_err(|e| format!("Parse error in stdlib/clorus/core.clr: {}", e))?;

            if debug {
                println!("   [DEBUG] Loaded {} expressions from stdlib", stdlib_exprs.len());
            }
            all_exprs.extend(stdlib_exprs);
        } else {
            // Try relative to compiler location
            let compiler_dir = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().and_then(|p| p.parent()).map(|p| p.to_path_buf()));

            if let Some(compiler_dir) = compiler_dir {
                let alt_stdlib = compiler_dir.join("stdlib/clorus/core.clr");
                if alt_stdlib.exists() {
                    let stdlib_source = fs::read_to_string(&alt_stdlib)
                        .map_err(|e| format!("Failed to read stdlib/clorus/core.clr: {}", e))?;

                    let stdlib_exprs = clorus::parse_and_expand(&stdlib_source)
                        .map_err(|e| format!("Parse error in stdlib/clorus/core.clr: {}", e))?;

                    all_exprs.extend(stdlib_exprs);
                }
            }
        }

        let transducers_path = Path::new("stdlib/clorus/transducers.clr");
        if transducers_path.exists() {
            let transducers_source = fs::read_to_string(transducers_path)
                .map_err(|e| format!("Failed to read stdlib/clorus/transducers.clr: {}", e))?;

            let transducers_exprs = clorus::parse_and_expand(&transducers_source)
                .map_err(|e| format!("Parse error in stdlib/clorus/transducers.clr: {}", e))?;

            if debug {
                println!("   [DEBUG] Loaded {} expressions from stdlib/clorus/transducers.clr", transducers_exprs.len());
            }
            all_exprs.extend(transducers_exprs);
        } else {
            // Try relative to compiler location
            let compiler_dir = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().and_then(|p| p.parent()).map(|p| p.to_path_buf()));

            if let Some(compiler_dir) = compiler_dir {
                let alt_transducers = compiler_dir.join("stdlib/clorus/transducers.clr");
                if alt_transducers.exists() {
                    let transducers_source = fs::read_to_string(&alt_transducers)
                        .map_err(|e| format!("Failed to read stdlib/clorus/transducers.clr: {}", e))?;

                    let transducers_exprs = clorus::parse_and_expand(&transducers_source)
                        .map_err(|e| format!("Parse error in stdlib/clorus/transducers.clr: {}", e))?;

                    all_exprs.extend(transducers_exprs);
                }
            }
        }
    } else if debug {
        println!("   [DEBUG] Stdlib loading disabled (build.stdlib = false)");
    }

    // Load and parse the user's entry file with module resolution
    let source = fs::read_to_string(entry_path)
        .map_err(|e| format!("Failed to read {}: {}", entry, e))?;

    // Parse entry file to find dependencies
    let entry_exprs = clorus::parse_and_expand(&source)
        .map_err(|e| format!("Parse error in {}: {}", entry, e))?;

    // Load all required modules recursively (skip .clip namespaces)
    let base_path = std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;

    let mut loaded_modules = HashSet::new();
    let mut module_exprs = Vec::new();

    // Build set of .clip namespace prefixes for fast lookup
    let mut clip_namespace_prefixes: HashSet<String> = HashSet::new();
    for package in &clip_packages {
        clip_namespace_prefixes.insert(package.name.clone());
    }

    // First pass: collect all modules to load (for summary)
    let mut modules_to_load = Vec::new();
    for expr in &entry_exprs {
        if let Expr::Require { specs } = expr {
            for spec in specs {
                let is_clip = clip_namespace_prefixes.iter().any(|prefix| {
                    spec.module.starts_with(prefix)
                });
                if !is_clip {
                    modules_to_load.push(spec.module.clone());
                }
            }
        }
        if let Expr::Ns { requires, .. } = expr {
            for spec in requires {
                let is_clip = clip_namespace_prefixes.iter().any(|prefix| {
                    spec.module.starts_with(prefix)
                });
                if !is_clip {
                    modules_to_load.push(spec.module.clone());
                }
            }
        }
    }

    // Show summary
    if !modules_to_load.is_empty() {
        if debug {
            println!("   Loading {} modules:", modules_to_load.len());
            for module in &modules_to_load {
                println!("      → {}", module);
            }
        } else {
            println!("   Loading {} module(s)...", modules_to_load.len());
        }
    }

    for expr in &entry_exprs {
        // Check top-level Expr::Require
        if let Expr::Require { specs } = expr {
            for spec in specs {
                // Check if this module is from a .clip package
                let is_clip = clip_namespace_prefixes.iter().any(|prefix| {
                    spec.module.starts_with(prefix)
                });

                if is_clip {
                    if debug {
                        println!("   Skipping module (from .clip): {}", spec.module);
                    }
                    continue;
                }

                let dep_exprs = load_module_recursive(&spec.module, &mut loaded_modules, &base_path, &clip_namespace_prefixes, debug)?;
                module_exprs.extend(dep_exprs);
            }
        }

        // Check Expr::Ns for embedded :require clauses
        if let Expr::Ns { requires, .. } = expr {
            for spec in requires {
                // Check if this module is from a .clip package
                let is_clip = clip_namespace_prefixes.iter().any(|prefix| {
                    spec.module.starts_with(prefix)
                });

                if is_clip {
                    if debug {
                        println!("   Skipping module (from .clip): {}", spec.module);
                    }
                    continue;
                }

                let dep_exprs = load_module_recursive(&spec.module, &mut loaded_modules, &base_path, &clip_namespace_prefixes, debug)?;
                module_exprs.extend(dep_exprs);
            }
        }
    }

    // Add module expressions (dependencies first)
    all_exprs.extend(module_exprs);

    // Add entry file expressions (after dependencies)
    all_exprs.extend(entry_exprs);

    if debug {
        println!("   [DEBUG] Total expressions to compile: {}", all_exprs.len());
    }

    if all_exprs.is_empty() {
        return Err("No expressions to compile".to_string());
    }

    // Compile to LLVM IR and create executable
    use inkwell::context::Context;
    use inkwell::targets::{Target, InitializationConfig, TargetMachine, RelocMode, CodeModel, FileType};
    use inkwell::OptimizationLevel;
    use clorus::CodeGen;

    let context = Context::create();
    let mut codegen = CodeGen::new(&context, &manifest.package.name);

    // Register .clip library exports with CodeGen (Phase 4)
    for package in &clip_packages {
        if debug {
            println!("   [DEBUG] Registering .clip package: {} v{}", package.name, package.version);
        }

        // Mark this namespace as coming from a .clip package
        // This prevents the compiler from looking for source files
        codegen.register_clip_namespace(&package.name);

        // TODO: Parse exports from package.exports and register symbols
        // For now, we just mark the namespace as external
    }

    // Register Rust FFI libraries with CodeGen
    for lib in &rust_ffi.libraries {
        use clorus::codegen::{RustLibrary, RustFunction, RustParam};

        let lib_name = lib.name.clone().replace("_ffi", "").replace('_', "-");

        let rust_lib = RustLibrary {
            name: lib_name,
            functions: lib.functions.iter().map(|f| RustFunction {
                name: f.name.clone(),
                params: f.params.iter().map(|p| RustParam {
                    name: p.name.clone(),
                    type_name: p.type_name.clone(),
                }).collect(),
                return_type: f.return_type.clone(),
            }).collect(),
        };

        codegen.register_rust_library(rust_lib);
    }

    // Compile each expression into a function
    let mut function_names = Vec::new();
    for (i, expr) in all_exprs.iter().enumerate() {
        // For libraries, prefix expr_ names with package name to avoid conflicts
        let fn_name = if lib_mode {
            format!("{}__expr_{}", manifest.package.name.replace('-', "_"), i)
        } else {
            format!("expr_{}", i)
        };
        codegen.wrap_in_function(expr, &fn_name)
            .map_err(|e| format!("Compile error: {}", e))?;
        function_names.push(fn_name);
    }

    // For libraries, create an initialization function that calls all expr functions
    // This ensures protocol registrations and other top-level code executes
    if lib_mode {
        let init_fn_name = format!("clorus_{}_init", manifest.package.name.replace('-', "_"));
        let value_ptr_type = context.i8_type().ptr_type(inkwell::AddressSpace::default());
        let init_fn_type = value_ptr_type.fn_type(&[], false);
        let init_fn = codegen.get_module().add_function(&init_fn_name, init_fn_type, None);

        let entry_block = context.append_basic_block(init_fn, "entry");
        let builder = codegen.get_builder();
        builder.position_at_end(entry_block);

        // Call each expr function to execute top-level forms
        for fn_name in &function_names {
            if let Some(func) = codegen.get_module().get_function(fn_name) {
                builder.build_call(func, &[], "init_call").unwrap();
            }
        }

        // Return nil
        let nil_fn = codegen.get_module().get_function("clorus_value_nil")
            .expect("clorus_value_nil should be declared");
        let nil_val = builder.build_call(nil_fn, &[], "init_nil").unwrap()
            .try_as_basic_value().left().unwrap().into_pointer_value();
        builder.build_return(Some(&nil_val)).unwrap();
    }

    // Only create main() for executables, not for libraries
    if !lib_mode {
        // Create a C-compatible main function that calls our expressions
        let main_fn_type = context.i32_type().fn_type(&[], false);
        let main_fn = codegen.get_module().add_function("main", main_fn_type, None);

    let entry_block = context.append_basic_block(main_fn, "entry");
    let builder = codegen.get_builder();
    builder.position_at_end(entry_block);

    // Declare printf for output
    let i8_ptr_type = context.i8_type().ptr_type(inkwell::AddressSpace::default());
    let printf_type = context.i32_type().fn_type(&[i8_ptr_type.into()], true);
    let printf_fn = codegen.get_module().add_function("printf", printf_type, None);

    // Create format strings for printing
    let float_format = builder.build_global_string_ptr("=> %g\n", "float_fmt").unwrap();

    // Look for -main function first to determine how to handle expressions
    // Construct the expected mangled name based on namespace
    let mut main_fn_name = "-main".to_string();

    // Check if there's a namespace declaration to construct qualified name
    for expr in &all_exprs {
        if let clorus_syntax::Expr::Ns { name, .. } = expr {
            if name != "user" {
                // Construct mangled name matching codegen.rs
                // Must replace BOTH dots and hyphens with underscores
                main_fn_name = format!("clorus_{}_{}",
                    name.replace('.', "_").replace('-', "_"),
                    "-main".replace('-', "_"));
            }
            break;
        }
    }

    let i8_ptr_type = context.i8_type().ptr_type(inkwell::AddressSpace::default());
    let main_fn_type = i8_ptr_type.fn_type(&[i8_ptr_type.into(), i8_ptr_type.into()], false);

    // First, call initialization functions of all loaded .clip packages
    // This ensures protocol registrations and other top-level code executes
    for package in &clip_packages {
        let init_fn_name = format!("clorus_{}_init", package.name.replace('-', "_"));

        // Declare the init function (it's external from the .clip package)
        let value_ptr_type = context.i8_type().ptr_type(inkwell::AddressSpace::default());
        let init_fn_type = value_ptr_type.fn_type(&[], false);

        if codegen.get_module().get_function(&init_fn_name).is_none() {
            codegen.get_module().add_function(&init_fn_name, init_fn_type, None);
        }

        // Call the init function
        if let Some(init_fn) = codegen.get_module().get_function(&init_fn_name) {
            if debug {
                println!("   [DEBUG] Calling {} init function", package.name);
            }
            builder.build_call(init_fn, &[], "init_clip").unwrap();
        }
    }

    // Call each compiled expression (defines functions, globals, etc.)
    let mut last_result: Option<inkwell::values::PointerValue> = None;

    for (i, fn_name) in function_names.iter().enumerate() {
        // Get the function from the module
        if let Some(func) = codegen.get_module().get_function(fn_name) {
            let result = builder.build_call(func, &[], "call").unwrap();
            let result_ptr = result.try_as_basic_value().left().unwrap().into_pointer_value();

            // Store last result in case we don't call -main
            last_result = Some(result_ptr);
        }
    }

    // Call -main if it exists (overrides last_result)
    if let Some(user_main_fn) = codegen.get_module().get_function(&main_fn_name) {
        // Call -main with empty args vector (like clorus run does)
        // We need to call clorus_vector_empty() at runtime to get an empty vector
        let vector_empty_fn = codegen.get_module()
            .get_function("clorus_vector_empty")
            .expect("clorus_vector_empty should be declared by runtime");

        let empty_vec_result = builder.build_call(vector_empty_fn, &[], "empty_vec").unwrap();
        let empty_vec_ptr = empty_vec_result.try_as_basic_value()
            .left()
            .expect("clorus_vector_empty should return a value")
            .into_pointer_value();

        // Call -main with the empty vector and NULL environment (no captured variables)
        let null_env = i8_ptr_type.const_null();
        let main_result = builder.build_call(user_main_fn, &[empty_vec_ptr.into(), null_env.into()], "call_main").unwrap();

        // Use the result from -main instead of last expression
        if let Some(result_val) = main_result.try_as_basic_value().left() {
            last_result = Some(result_val.into_pointer_value());
        }
    }

    // Print the final result if we have one (from -main or last expression)
    if let Some(result_ptr) = last_result {
        // Get the print_value function which handles all types
        let print_value_fn = codegen.get_module()
            .get_function("clorus_print_value")
            .expect("clorus_print_value should be declared by runtime");

        builder.build_call(
            print_value_fn,
            &[result_ptr.into()],
            "print_result"
        ).unwrap();
    }

    // Return 0 (success)
    let zero = context.i32_type().const_int(0, false);
    builder.build_return(Some(&zero)).unwrap();
    } // end if !lib_mode

    // Create target directory if it doesn't exist
    let target_dir = Path::new("target");
    if !target_dir.exists() {
        fs::create_dir(target_dir)
            .map_err(|e| format!("Failed to create target directory: {}", e))?;
    }

    // Initialize LLVM targets
    Target::initialize_native(&InitializationConfig::default())
        .map_err(|e| format!("Failed to initialize LLVM target: {}", e))?;

    let target_triple = TargetMachine::get_default_triple();
    let target = Target::from_triple(&target_triple)
        .map_err(|e| format!("Failed to get target: {}", e))?;

    let target_machine = target
        .create_target_machine(
            &target_triple,
            "generic",
            "",
            OptimizationLevel::Default,
            RelocMode::Default,
            CodeModel::Default,
        )
        .ok_or("Failed to create target machine")?;

    // Write object file
    let obj_path = target_dir.join(format!("{}.o", manifest.package.name));
    target_machine
        .write_to_file(codegen.get_module(), FileType::Object, &obj_path)
        .map_err(|e| format!("Failed to write object file: {}", e))?;

    println!("    Generated object file: {}", obj_path.display());

    // For library mode, also generate LLVM bitcode for JIT compatibility
    if lib_mode {
        let bc_path = target_dir.join(format!("{}.bc", manifest.package.name));
        codegen.get_module().write_bitcode_to_path(&bc_path);
        println!("    Generated bitcode file: {}", bc_path.display());
        println!("    Finished lib build in 0.00s");
        return Ok(());
    }

    // Link the object file into an executable
    let exe_path = target_dir.join(&manifest.package.name);

    // Find runtime library.
    // Prefer locally built runtime first to keep compiler/runtime symbols in sync
    // during development and tests, then fall back to CLORUS_HOME installs.
    let mut runtime_lib: Option<String> = None;

    // If the CLI itself is a debug build, prefer debug runtime archives even without --debug.
    // This prevents stale release archives from causing unresolved symbols during dev/test loops.
    let prefer_debug_runtime = debug || cfg!(debug_assertions);
    let local_candidates: Vec<&str> = if prefer_debug_runtime {
        vec![
            "target/debug/deps/libclorus_runtime.a",
            "target/debug/libclorus_runtime.a",
            "target/release/deps/libclorus_runtime.a",
            "target/release/libclorus_runtime.a",
            "../target/debug/deps/libclorus_runtime.a",
            "../target/debug/libclorus_runtime.a",
            "../target/release/deps/libclorus_runtime.a",
            "../target/release/libclorus_runtime.a",
            "../../target/debug/deps/libclorus_runtime.a",
            "../../target/debug/libclorus_runtime.a",
            "../../target/release/deps/libclorus_runtime.a",
            "../../target/release/libclorus_runtime.a",
        ]
    } else {
        vec![
            "target/release/deps/libclorus_runtime.a",
            "target/release/libclorus_runtime.a",
            "target/debug/deps/libclorus_runtime.a",
            "target/debug/libclorus_runtime.a",
            "../target/release/deps/libclorus_runtime.a",
            "../target/release/libclorus_runtime.a",
            "../target/debug/deps/libclorus_runtime.a",
            "../target/debug/libclorus_runtime.a",
            "../../target/release/deps/libclorus_runtime.a",
            "../../target/release/libclorus_runtime.a",
            "../../target/debug/deps/libclorus_runtime.a",
            "../../target/debug/libclorus_runtime.a",
        ]
    };
    for path in local_candidates {
        if Path::new(path).exists() {
            runtime_lib = Some(path.to_string());
            break;
        }
    }

    // Installed runtime fallback via CLORUS_HOME
    if runtime_lib.is_none() {
        if let Ok(clorus_home) = std::env::var("CLORUS_HOME") {
            let lib_dir = Path::new(&clorus_home).join("lib");
            let candidates = [
                lib_dir.join("libclorus_runtime.a"),
                lib_dir.join("libclorus_runtime.dylib"),
                lib_dir.join("libclorus_runtime.rlib"),
            ];
            for path in candidates {
                if path.exists() {
                    runtime_lib = Some(path.to_string_lossy().to_string());
                    break;
                }
            }
        }
    }

    // Try to find relative to clorus executable (for installed version or running from workspace)
    if runtime_lib.is_none() {
        if let Ok(exe_path) = std::env::current_exe() {
            // Resolve symlinks
            let exe_path = if let Ok(canonical) = exe_path.canonicalize() {
                canonical
            } else {
                exe_path
            };

            if let Some(exe_dir) = exe_path.parent() {
                // Try same directory as executable first
                let same_dir = exe_dir.join("libclorus_runtime.a");
                if same_dir.exists() {
                    runtime_lib = Some(same_dir.to_string_lossy().to_string());
                }

                // Try relative to executable: deps/libclorus_runtime.a (same directory as exe)
                if runtime_lib.is_none() {
                    let same_dir_deps = exe_dir.join("deps/libclorus_runtime.a");
                    if same_dir_deps.exists() {
                        runtime_lib = Some(same_dir_deps.to_string_lossy().to_string());
                    }
                }

                // Try relative to workspace root: ../target/{release,debug}/deps/
                // This handles when clorus is run from target/release/clorus
                // exe_dir = /path/to/workspace/target/release
                // workspace_root = /path/to/workspace
                if runtime_lib.is_none() {
                    if let Some(target_dir) = exe_dir.parent() {
                        // target_dir = /path/to/workspace/target
                        if target_dir.file_name().and_then(|n| n.to_str()) == Some("target") {
                            if let Some(workspace_root) = target_dir.parent() {
                                // workspace_root = /path/to/workspace
                                for subpath in &[
                                    "target/release/deps/libclorus_runtime.a",
                                    "target/release/libclorus_runtime.a",
                                    "target/debug/deps/libclorus_runtime.a",
                                    "target/debug/libclorus_runtime.a",
                                ] {
                                    let path = workspace_root.join(subpath);
                                    if path.exists() {
                                        runtime_lib = Some(path.to_string_lossy().to_string());
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }

                // Try ../lib relative to bin directory (installation layout)
                if runtime_lib.is_none() {
                    let lib_path = exe_dir.parent().map(|p| p.join("lib/libclorus_runtime.a"));
                    if let Some(path) = lib_path {
                        if path.exists() {
                            runtime_lib = Some(path.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
    }

    let runtime_lib = runtime_lib.ok_or_else(|| {
        "Runtime library not found.\n\
        Expected location: $CLORUS_HOME/lib/libclorus_runtime.*\n\
        \n\
        To fix this:\n\
        1. Build the runtime: cargo build -p clorus-runtime --release\n\
        2. Install it: scripts/install/install.sh (sets CLORUS_HOME)\n\
        3. Or set CLORUS_HOME to your Clorus install directory".to_string()
    })?;

    // Find example-rust-lib library (optional - only needed if using rust.example)
    let mut example_lib: Option<String> = None;
    for path in &[
        "target/release/libexample_rust_lib.a",
        "target/debug/libexample_rust_lib.a",
        "../target/release/libexample_rust_lib.a",
        "../target/debug/libexample_rust_lib.a",
        "../../target/release/libexample_rust_lib.a",
        "../../target/debug/libexample_rust_lib.a",
    ] {
        if Path::new(path).exists() {
            example_lib = Some(path.to_string());
            break;
        }
    }

    // Link using clang/cc
    let mut link_cmd = std::process::Command::new("cc");
    link_cmd
        .arg(&obj_path)
        .arg(runtime_lib);

    // Add Rust FFI libraries
    for lib_path in rust_ffi.get_static_lib_paths() {
        link_cmd.arg(lib_path);
    }

    // Add .clip library object files (Phase 3: automatic linking)
    for package in &clip_packages {
        if let Some(ref object_path) = package.object_path {
            println!("      Linking {} v{}", package.name, package.version);
            link_cmd.arg(object_path);
        } else if let Some(ref bitcode_path) = package.bitcode_path {
            println!("      Linking {} v{} (bitcode)", package.name, package.version);
            link_cmd.arg(bitcode_path);
        }
    }

    // Add output path
    link_cmd.arg("-o").arg(&exe_path);

    // Link C++ standard library (for LLVM runtime)
    link_cmd.arg("-lc++");

    // Add system libraries from manifest
    for lib in &manifest.link.libraries {
        link_cmd.arg(format!("-l{}", lib));
    }

    // Add macOS frameworks (required for GUI libraries like egui)
    #[cfg(target_os = "macos")]
    {
        // Default frameworks always needed for LLVM/Clorus
        let mut frameworks = vec![
            "CoreFoundation".to_string(),
            "Security".to_string(),
        ];

        // Add user-specified frameworks from Clorus.toml
        frameworks.extend(manifest.link.frameworks.clone());

        // Remove duplicates
        frameworks.sort();
        frameworks.dedup();

        for framework in frameworks {
            link_cmd.arg("-framework").arg(framework);
        }
    }

    let link_status = link_cmd
        .status()
        .map_err(|e| format!("Failed to link: {}", e))?;

    if !link_status.success() {
        return Err("Linking failed".to_string());
    }

    println!("    Finished dev [unoptimized] target(s) in 0.00s");
    println!();
    println!("   Executable: {}", exe_path.display());
    println!("   Run with: ./{}", exe_path.display());

    Ok(())
}

/// Extract required module names from expressions
fn extract_required_modules(exprs: &[Expr]) -> Vec<String> {
    let mut modules = Vec::new();

    for expr in exprs {
        match expr {
            Expr::Ns { requires, .. } => {
                for req_spec in requires {
                    modules.push(req_spec.module.clone());
                }
            }
            Expr::Require { specs } => {
                for spec in specs {
                    modules.push(spec.module.clone());
                }
            }
            _ => {}
        }
    }

    modules
}

/// Recursively load and compile required modules
fn load_and_compile_modules<'ctx>(
    modules: &[String],
    codegen: &mut clorus::CodeGen<'ctx>,
    loaded: &mut HashSet<String>,
    project_root: &Path,
    source_dirs: &[String],
) -> Result<(), String> {
    use clorus_codegen::namespace_context::{ImportBinding, NamespaceContext};

    for module_name in modules {
        if loaded.contains(module_name) {
            continue; // Already loaded
        }

        // Convert module name to file path: math -> math.clrs/math.clr, text-field.core -> text-field/core.clrs
        // We preserve hyphens in the file path (e.g., text-field.core -> text-field/core.clrs)
        let module_path = module_name.replace('.', "/");

        // Try resolving clorus.* stdlib namespaces first.
        let mut module_file = if module_name.starts_with("clorus.") {
            namespace_to_path(module_name, project_root).ok()
        } else {
            None
        };

        // For project namespaces, try each source directory in order.
        if module_file.is_none() {
            for src_dir in source_dirs {
                // Try .clrs first (source files)
                let candidate_clrs = project_root
                    .join(src_dir)
                    .join(format!("{}.clrs", module_path));
                if candidate_clrs.exists() {
                    module_file = Some(candidate_clrs);
                    break;
                }

                // Fall back to .clr (library/stdlib files)
                let candidate_clr = project_root
                    .join(src_dir)
                    .join(format!("{}.clr", module_path));
                if candidate_clr.exists() {
                    module_file = Some(candidate_clr);
                    break;
                }
            }
        }

        let module_file = module_file.ok_or_else(|| {
            format!(
                "Module '{}' not found. Searched in: {}",
                module_name,
                source_dirs
                    .iter()
                    .map(|d| format!("{}/{}.clrs or {}/{}.clr", d, module_path, d, module_path))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;

        // Read and parse the module
        let source = fs::read_to_string(&module_file)
            .map_err(|e| format!("Failed to read {}: {}", module_file.display(), e))?;

        let exprs = clorus::parse_and_expand(&source)
            .map_err(|e| format!("Parse error in {}: {}", module_file.display(), e))?;

        // Extract and load transitive dependencies first
        let sub_modules = extract_required_modules(&exprs);
        if !sub_modules.is_empty() {
            load_and_compile_modules(&sub_modules, codegen, loaded, project_root, source_dirs)?;
        }

        // Compile this module's expressions
        for expr in &exprs {
            // Set namespace context if this is an ns declaration
            if let Expr::Ns { name, requires, rust_imports } = expr {
                // Validate that namespace matches file path (like Clojure)
                // Note: Clojure convention is underscore in filesystem, hyphen in namespace
                // e.g., src/my_module/core.clrs -> (ns my-module.core)
                let expected_ns_with_underscores = module_name;
                let expected_ns_with_hyphens = module_name.replace('_', "-");

                if name != expected_ns_with_underscores && name != &expected_ns_with_hyphens {
                    eprintln!(
                        "Warning: Namespace mismatch in {}:\n\n  \
                         Expected: (ns {}) or (ns {})\n  \
                         Found:    (ns {})\n\n  \
                         Clorus follows Clojure conventions, but does not require namespace/path equality.",
                        module_file.display(),
                        expected_ns_with_hyphens,
                        expected_ns_with_underscores,
                        name
                    );
                    eprintln!(
                        "  Hint: Rename namespace to {} for consistency.",
                        expected_ns_with_hyphens
                    );
                }
                /*
                    return Err(format!(
                        "Namespace mismatch in {}:\n\n  \
                         Expected: (ns {}) or (ns {})\n  \
                         Found:    (ns {})\n\n  \
                         In Clorus, the namespace must match the file path.\n  \
                         Note: Use hyphens (-) in namespaces for Clojure-style, underscores (_) match filesystem.\n\n  \
                         Fix by either:\n  \
                         1. Change namespace to: (ns {}) [Clojure-style]\n  \
                         2. Change namespace to: (ns {}) [Direct match]\n  \
                         3. Move file to: src/{}.clrs",
                        module_file.display(),
                        expected_ns_with_hyphens,
                        expected_ns_with_underscores,
                        name,
                        expected_ns_with_hyphens,
                        expected_ns_with_underscores,
                        name.replace('.', "/").replace('-', "_")
                    ));
                */

                let mut ns_ctx = NamespaceContext::default_namespace();
                ns_ctx.current = name.clone();

                // Process requires
                for req_spec in requires {
                    if let Some(ref alias) = req_spec.alias {
                        ns_ctx.aliases.insert(alias.clone(), req_spec.module.clone());
                    }
                    for symbol in &req_spec.refer {
                        ns_ctx.imports.insert(
                            symbol.clone(),
                            ImportBinding {
                                namespace: req_spec.module.clone(),
                                symbol: symbol.clone(),
                            },
                        );
                    }
                    for (source_symbol, local_symbol) in &req_spec.rename {
                        ns_ctx.imports.insert(
                            local_symbol.clone(),
                            ImportBinding {
                                namespace: req_spec.module.clone(),
                                symbol: source_symbol.clone(),
                            },
                        );
                    }
                }

                // Process rust imports
                for rust_import in rust_imports {
                    if let Some(ref alias) = rust_import.alias {
                        let rust_module = format!("rust.{}", rust_import.library);
                        ns_ctx.aliases.insert(alias.clone(), rust_module);
                    }
                }

                codegen.set_namespace(ns_ctx);
            }

            // Compile the expression (defn, def, etc.)
            let fn_name = format!("mod_{}_{}", module_name.replace('.', "_"), loaded.len());
            codegen.wrap_in_function(expr, &fn_name)
                .map_err(|e| format!("Compile error in {}: {}", module_name, e))?;
        }

        loaded.insert(module_name.clone());
    }

    Ok(())
}


pub fn run(debug: bool, use_jit: bool, extra_args: Vec<String>) -> Result<(), String> {
    if use_jit {
        // Default path: JIT execution.
        if debug {
            println!("   Mode: JIT execution (default)");
            println!();
        }
        run_jit_internal(debug, extra_args)
    } else {
        // Legacy fallback path: compile + run executable
        if debug {
            println!("   Mode: Legacy compile+run (--legacy-run)");
            println!();
        }

        // Build the project first
        build_with_debug(debug)?;

        // Get the manifest to know the executable name
        let manifest = Manifest::find_in_current_dir()?;
        let exe_path = format!("./target/{}", manifest.package.name);

        println!();
        println!("     Running `{}`", exe_path);
        println!();

        // Execute the binary with any extra arguments
        let status = std::process::Command::new(&exe_path)
            .args(&extra_args)
            .status()
            .map_err(|e| format!("Failed to execute {}: {}", exe_path, e))?;

        if !status.success() {
            return Err(format!("Process exited with status: {}", status));
        }

        Ok(())
    }
}

/// JIT execution mode used by `clorus run` default and REPL flows.
fn run_jit_internal(debug: bool, extra_args: Vec<String>) -> Result<(), String> {
    let manifest = Manifest::find_in_current_dir()?;

    // For run, we need an explicit entry point (not src/lib.clrs)
    // Libraries with src/lib.clrs shouldn't be run directly
    let entry = if let Some(override_entry) = get_entry_override() {
        override_entry
    } else {
        match &manifest.build.entry {
            Some(e) => e.clone(),
            None => {
                return Err("Cannot run library package (no entry point specified)\nLibraries use src/lib.clrs and cannot be run directly. Specify entry in Clorus.toml or create a test/example file.".to_string());
            }
        }
    };

    let entry_path = Path::new(&entry);

    if !entry_path.exists() {
        return Err(format!("Entry file '{}' not found", entry));
    }

    // Process Rust dependencies (auto-generate FFI and compile)
    let rust_ffi = crate::rust_ffi::RustFfiProcessor::process_dependencies(&manifest, debug)?;

    if debug {
        println!("   [DEBUG] Processed {} Rust libraries", rust_ffi.libraries.len());
        for lib in &rust_ffi.libraries {
            println!("   [DEBUG]   - {} with {} functions", lib.name, lib.functions.len());
        }
    }

    if debug {
        println!("   Debug mode: enabled");
        println!("   Note: Memory tracking requires runtime Value* types");
        println!("   Currently using f64 - full tracking coming soon!");
        println!();
    }

    // Load .clip dependencies automatically (Phase 3)
    if debug {
        println!("   Resolving dependencies...");
    }
    let clip_packages = crate::pack::load_clip_dependencies(&manifest)?;

    if !clip_packages.is_empty() && debug {
        println!("   Loaded {} .clip package(s)", clip_packages.len());
        for package in &clip_packages {
            println!("      ✓ {} v{}", package.name, package.version);
        }
    }

    println!("   Compiling {} v{}", manifest.package.name, manifest.package.version);

    // Load and parse stdlib/clorus/core.clr + stdlib/clorus/transducers.clr (optional)
    let mut all_exprs = Vec::new();
    if manifest.build.stdlib {
        let stdlib_path = Path::new("stdlib/clorus/core.clr");
        if stdlib_path.exists() {
            let stdlib_source = fs::read_to_string(stdlib_path)
                .map_err(|e| format!("Failed to read stdlib/clorus/core.clr: {}", e))?;

            let stdlib_exprs = clorus::parse_and_expand(&stdlib_source)
                .map_err(|e| format!("Parse error in stdlib/clorus/core.clr: {}", e))?;

            if debug {
                println!("   [DEBUG] Loaded {} expressions from stdlib", stdlib_exprs.len());
            }
            all_exprs.extend(stdlib_exprs);
        } else if debug {
            println!("   [DEBUG] stdlib/clorus/core.clr not found, stdlib functions unavailable");
        }

        let transducers_path = Path::new("stdlib/clorus/transducers.clr");
        if transducers_path.exists() {
            let transducers_source = fs::read_to_string(transducers_path)
                .map_err(|e| format!("Failed to read stdlib/clorus/transducers.clr: {}", e))?;

            let transducers_exprs = clorus::parse_and_expand(&transducers_source)
                .map_err(|e| format!("Parse error in stdlib/clorus/transducers.clr: {}", e))?;

            if debug {
                println!("   [DEBUG] Loaded {} expressions from stdlib/clorus/transducers.clr", transducers_exprs.len());
            }
            all_exprs.extend(transducers_exprs);
        } else if debug {
            println!("   [DEBUG] stdlib/clorus/transducers.clr not found, transducers unavailable");
        }
    } else if debug {
        println!("   [DEBUG] Stdlib loading disabled (build.stdlib = false)");
    }

    // Load entry file
    let source = fs::read_to_string(entry_path)
        .map_err(|e| format!("Failed to read {}: {}", entry, e))?;

    // Parse and expand macros
    let exprs = clorus::parse_and_expand(&source)
        .map_err(|e| format!("Parse error: {}", e))?;

    // Add entry exprs to all_exprs
    all_exprs.extend(exprs);

    if all_exprs.is_empty() {
        return Err("No expressions to execute".to_string());
    }

    println!("    Finished dev [unoptimized] target(s) in 0.00s");
    println!("     Running `{}`", entry);
    println!();

    // Load clorus-runtime library FIRST (required for Value* operations)
    let _runtime_lib = match load_runtime_library() {
        Ok(lib) => {
            if debug {
                println!("   [DEBUG] Loaded clorus-runtime library");
            }
            Some(lib)
        }
        Err(e) => {
            if debug {
                println!("   [DEBUG] clorus-runtime not loaded: {}", e);
                println!("   [DEBUG] String and collection operations may not work");
            }
            None
        }
    };

    // Load clorus-std library for rust.fs functions
    // This makes clorus_fs_* symbols available to the JIT
    let _std_lib = match load_std_library() {
        Ok(lib) => {
            if debug {
                println!("   [DEBUG] Loaded clorus-std library");
            }
            Some(lib)
        }
        Err(e) => {
            // Library not found - that's OK if not using fs functions
            if debug {
                println!("   [DEBUG] clorus-std not loaded: {}", e);
                println!("   [DEBUG] rust.fs functions will not be available");
            }
            None
        }
    };

    // Load Rust FFI libraries automatically
    let mut _rust_libs = Vec::new();
    for lib_path in rust_ffi.get_dynamic_lib_paths() {
        match load_dynamic_library(&lib_path) {
            Ok(lib) => {
                if debug {
                    println!("   [DEBUG] Loaded Rust library: {}", lib_path.display());
                }
                _rust_libs.push(lib);
            }
            Err(e) => {
                if debug {
                    println!("   [DEBUG] Failed to load {}: {}", lib_path.display(), e);
                }
            }
        }
    }

    // Load clorus-core library for Clojure-style functions (slurp, spit)
    let _core_lib = match load_core_library() {
        Ok(lib) => {
            if debug {
                println!("   [DEBUG] Loaded clorus-core library");
            }
            Some(lib)
        }
        Err(e) => {
            if debug {
                println!("   [DEBUG] clorus-core not loaded: {}", e);
                println!("   [DEBUG] slurp/spit functions will not be available");
            }
            None
        }
    };

    // Compile and execute with JIT - similar to REPL engine
    use inkwell::context::Context;
    use inkwell::OptimizationLevel;
    use clorus::CodeGen;
    use clorus_runtime::value::{clorus_value_long, clorus_release};

    // Ensure runtime symbols are linked by touching them
    unsafe {
        let _dummy = clorus_value_long(0);
        clorus_release(_dummy);
    }

    let context = Context::create();
    let mut codegen = CodeGen::new(&context, &manifest.package.name);

    // Load and link .clip library bitcode FIRST (Phase 4 JIT support)
    // This must happen before compiling user code so symbols are available
    let mut successfully_loaded_clip = Vec::new();
    let mut failed_clip = Vec::new();

    for package in &clip_packages {
        if debug {
            println!("   [DEBUG] Loading .clip package: {} v{}", package.name, package.version);
        }

        // Mark this namespace as coming from a .clip package
        // This prevents the compiler from looking for source files
        codegen.register_clip_namespace(&package.name);

        // Load bitcode if available (for JIT execution)
        if let Some(ref bc_path) = package.bitcode_path {
            if debug {
                println!("   [DEBUG] Loading bitcode from {}", bc_path.display());
            }

            // Read bitcode file
            match fs::read(bc_path) {
                Ok(bc_data) => {
                    // Create memory buffer from bitcode data
                    let mem_buf = inkwell::memory_buffer::MemoryBuffer::create_from_memory_range(&bc_data, "clip_bitcode");

                    // Parse bitcode module
                    match inkwell::module::Module::parse_bitcode_from_buffer(&mem_buf, &context) {
                        Ok(bc_module) => {
                            // Link into main module
                            match codegen.get_module().link_in_module(bc_module) {
                                Ok(_) => {
                                    if debug {
                                        println!("   [DEBUG] Successfully linked {} bitcode", package.name);
                                    }
                                    successfully_loaded_clip.push(package.name.clone());
                                }
                                Err(e) => {
                                    if debug {
                                        println!("   [DEBUG] Failed to link bitcode: {}", e);
                                    }
                                    failed_clip.push(package.name.clone());
                                }
                            }
                        }
                        Err(e) => {
                            if debug {
                                println!("   [DEBUG] Failed to parse bitcode: {}", e);
                            }
                            failed_clip.push(package.name.clone());
                        }
                    }
                }
                Err(e) => {
                    if debug {
                        println!("   [DEBUG] Failed to read bitcode file: {}", e);
                    }
                    failed_clip.push(package.name.clone());
                }
            }
        } else {
            failed_clip.push(package.name.clone());
        }
    }

    // If any .clip packages failed to load, we can't run in JIT mode
    if !failed_clip.is_empty() {
        eprintln!();
        eprintln!("⚠️  JIT execution limitation:");
        eprintln!("   Could not load the following .clip packages: {}", failed_clip.join(", "));
        eprintln!();
        eprintln!("   'clorus run' uses JIT compilation and has limitations with .clip dependencies.");
        eprintln!("   The bitcode format may be incompatible or unavailable.");
        eprintln!();
        eprintln!("   Solution:");
        eprintln!("   • Use 'clorus build' to create an executable");
        eprintln!("   • Run the compiled binary: ./target/{}", manifest.package.name);
        eprintln!();
        eprintln!("   (Compiled binaries work perfectly with .clip dependencies!)");
        return Err(format!("Cannot load .clip packages in JIT mode: {}", failed_clip.join(", ")));
    }

    // Register Rust FFI libraries with CodeGen
    for lib in &rust_ffi.libraries {
        use clorus::codegen::{RustLibrary, RustFunction, RustParam};

        let lib_name = lib.name.clone().replace("_ffi", "").replace('_', "-");

        if debug {
            println!("   [DEBUG] Registering Rust library: {} (orig: {}, functions: {})",
                lib_name, lib.name, lib.functions.len());
            for func in &lib.functions {
                println!("   [DEBUG]   - {}({}) -> {}", func.name,
                    func.params.iter().map(|p| &p.type_name).cloned().collect::<Vec<_>>().join(", "),
                    func.return_type);
            }
        }

        let rust_lib = RustLibrary {
            name: lib_name,
            functions: lib.functions.iter().map(|f| RustFunction {
                name: f.name.clone(),
                params: f.params.iter().map(|p| RustParam {
                    name: p.name.clone(),
                    type_name: p.type_name.clone(),
                }).collect(),
                return_type: f.return_type.clone(),
            }).collect(),
        };

        codegen.register_rust_library(rust_lib);
    }

    // Compile stdlib expressions FIRST so they're available to modules
    // We need to track how many stdlib expressions there are so modules can use them
    let mut stdlib_expr_count = 0usize;
    if manifest.build.stdlib {
        let stdlib_path = Path::new("stdlib/clorus/core.clr");
        if stdlib_path.exists() {
            if let Ok(stdlib_source) = fs::read_to_string(stdlib_path) {
                if let Ok(stdlib_exprs) = clorus::parse_and_expand(&stdlib_source) {
                    stdlib_expr_count += stdlib_exprs.len();
                }
            }
        }

        let transducers_path = Path::new("stdlib/clorus/transducers.clr");
        if transducers_path.exists() {
            if let Ok(transducers_source) = fs::read_to_string(transducers_path) {
                if let Ok(transducers_exprs) = clorus::parse_and_expand(&transducers_source) {
                    stdlib_expr_count += transducers_exprs.len();
                }
            }
        }
    }

    if debug {
        println!("   [DEBUG] Compiling {} stdlib expressions before modules", stdlib_expr_count);
    }

    // Compile stdlib expressions into CodeGen so modules can use them
    // Use "user" namespace for stdlib (no mangling)
    use clorus_codegen::namespace_context::{ImportBinding, NamespaceContext};
    use std::collections::HashMap;
    let stdlib_ns = NamespaceContext {
        current: "user".to_string(),
        aliases: HashMap::new(),
        imports: HashMap::new(),
    };
    codegen.set_namespace(stdlib_ns);

    for (i, expr) in all_exprs.iter().take(stdlib_expr_count).enumerate() {
        // Skip namespace declarations
        if matches!(expr, Expr::Ns { .. }) {
            continue;
        }

        // Skip declare statements
        if let Expr::Declare { names } = expr {
            for name in names {
                codegen.add_forward_declaration(name);
            }
            continue;
        }

        let fn_name = format!("stdlib_init_{}", i);
        codegen.wrap_in_function(expr, &fn_name)
            .map_err(|e| format!("Compile error in stdlib: {}", e))?;
    }

    if debug {
        println!("   [DEBUG] Stdlib expressions compiled successfully");
    }

    // Build set of .clip namespace prefixes for fast lookup
    let mut clip_namespace_prefixes: HashSet<String> = HashSet::new();
    for package in &clip_packages {
        clip_namespace_prefixes.insert(package.name.clone());
    }

    // Load and compile required modules (they can now use stdlib functions)
    let project_root = std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;

    let required_modules = extract_required_modules(&all_exprs);

    // Filter out .clip namespaces (they're already compiled in .clip packages)
    let source_modules: Vec<String> = required_modules.into_iter()
        .filter(|module_name| {
            // Check if this module is from a .clip package
            let is_clip = clip_namespace_prefixes.iter().any(|prefix| {
                module_name.starts_with(prefix)
            });

            if is_clip && debug {
                println!("   [DEBUG] Skipping module (from .clip): {}", module_name);
            }

            !is_clip
        })
        .collect();

    let mut loaded_modules = HashSet::new();

    // Get source directories from manifest, default to ["src"] if not specified
    let source_dirs: Vec<String> = if manifest.build.src.is_empty() {
        vec!["src".to_string()]
    } else {
        manifest.build.src.clone()
    };

    if !source_modules.is_empty() {
        load_and_compile_modules(&source_modules, &mut codegen, &mut loaded_modules, &project_root, &source_dirs)?;
    }

    // Update namespace context for entry file
    for expr in &all_exprs {
        if let Expr::Ns { name, requires, rust_imports } = expr {
            // Validate that namespace matches entry file path (like Clojure)
            // Calculate expected namespace from file path
            let expected_ns_with_underscores = entry_path
                .strip_prefix("src/")
                .or_else(|_| entry_path.strip_prefix("src\\"))
                .unwrap_or(entry_path)
                .with_extension("")
                .to_string_lossy()
                .replace('/', ".")
                .replace('\\', ".");

            // Clojure convention: underscores in filesystem, hyphens in namespace
            let expected_ns_with_hyphens = expected_ns_with_underscores.replace('_', "-");

            if name != &expected_ns_with_underscores && name != &expected_ns_with_hyphens {
                eprintln!(
                    "Warning: Namespace mismatch in {}:\n\n  \
                     Expected: (ns {}) or (ns {})\n  \
                     Found:    (ns {})\n\n  \
                     Clorus follows Clojure conventions, but does not require namespace/path equality.",
                    entry_path.display(),
                    expected_ns_with_hyphens,
                    expected_ns_with_underscores,
                    name
                );
                if debug {
                    eprintln!(
                        "  Hint: Rename namespace to {} for consistency.",
                        expected_ns_with_hyphens
                    );
                }
            }
            /*
                return Err(format!(
                    "Namespace mismatch in {}:\n\n  \
                     Expected: (ns {}) or (ns {})\n  \
                     Found:    (ns {})\n\n  \
                     In Clorus, the namespace must match the file path.\n  \
                     Note: Use hyphens (-) in namespaces for Clojure-style, underscores (_) match filesystem.\n\n  \
                     Fix by either:\n  \
                     1. Change namespace to: (ns {}) [Clojure-style]\n  \
                     2. Change namespace to: (ns {}) [Direct match]\n  \
                     3. Move entry file to: src/{}.clrs",
                    entry_path.display(),
                    expected_ns_with_hyphens,
                    expected_ns_with_underscores,
                    name,
                    expected_ns_with_hyphens,
                    expected_ns_with_underscores,
                    name.replace('.', "/").replace('-', "_")
                ));
            */

            let mut ns_ctx = NamespaceContext::default_namespace();
            ns_ctx.current = name.clone();

            // Process requires
            for req_spec in requires {
                if let Some(ref alias) = req_spec.alias {
                    ns_ctx.aliases.insert(alias.clone(), req_spec.module.clone());
                }
                for symbol in &req_spec.refer {
                    ns_ctx.imports.insert(
                        symbol.clone(),
                        ImportBinding {
                            namespace: req_spec.module.clone(),
                            symbol: symbol.clone(),
                        },
                    );
                }
                for (source_symbol, local_symbol) in &req_spec.rename {
                    ns_ctx.imports.insert(
                        local_symbol.clone(),
                        ImportBinding {
                            namespace: req_spec.module.clone(),
                            symbol: source_symbol.clone(),
                        },
                    );
                }
            }

            // Process rust imports
            for rust_import in rust_imports {
                if let Some(ref alias) = rust_import.alias {
                    let rust_module = format!("rust.{}", rust_import.library);
                    ns_ctx.aliases.insert(alias.clone(), rust_module);
                }
            }

            codegen.set_namespace(ns_ctx);
            break; // Only process first ns declaration
        }
    }

    let mut function_names = Vec::new();

    // Compile all expressions
    for (i, expr) in all_exprs.iter().enumerate() {
        let fn_name = format!("expr_{}", i);
        codegen.wrap_in_function(expr, &fn_name)
            .map_err(|e| format!("Compile error: {}", e))?;
        function_names.push(fn_name);
    }

    // Create JIT engine
    let engine = codegen.get_module()
        .create_jit_execution_engine(OptimizationLevel::None)
        .map_err(|e| format!("JIT error: {}", e))?;

    // First, call initialization functions of all loaded .clip packages
    // This ensures protocol registrations and other top-level code executes
    for package in &successfully_loaded_clip {
        let init_fn_name = format!("clorus_{}_init", package.replace('-', "_"));

        if debug {
            println!("   [DEBUG] Calling {} init function in JIT mode", package);
        }

        unsafe {
            type InitFunc = unsafe extern "C" fn() -> *mut u8;
            match engine.get_function::<InitFunc>(&init_fn_name) {
                Ok(init_fn) => {
                    init_fn.call();
                    if debug {
                        println!("   [DEBUG] Successfully called {} init", package);
                    }
                }
                Err(e) => {
                    if debug {
                        println!("   [DEBUG] Warning: Could not find init function {}: {}", init_fn_name, e);
                    }
                }
            }
        }
    }

    // Execute expressions that define globals/functions (def, defn)
    // Then execute all expressions and print the last result
    let mut last_result_ptr: *mut u8 = std::ptr::null_mut();
    for (i, fn_name) in function_names.iter().enumerate() {
        unsafe {
            type EvalFunc = unsafe extern "C" fn() -> *mut u8;
            let jit_fn = engine.get_function::<EvalFunc>(fn_name)
                .map_err(|e| format!("Function not found: {}", e))?;
            let result = jit_fn.call();

            // Store the result from the last expression
            if i == function_names.len() - 1 {
                last_result_ptr = result;
            }
        }
    }

    // Look for -main function and call it with args if present
    // Need to check both simple name and namespace-qualified name
    let mut main_fn_name = "-main".to_string();

    // If there's a namespace in the first expression, construct qualified name
    for expr in &all_exprs {
        if let Expr::Ns { name, .. } = expr {
            if name != "user" {
                // Construct mangled name matching codegen.rs
                // Must replace BOTH dots and hyphens with underscores
                main_fn_name = format!("clorus_{}_{}",
                    name.replace('.', "_").replace('-', "_"),
                    "-main".replace('-', "_"));
            }
            break;
        }
    }

    if debug {
        println!("   [DEBUG] Looking for main function: {}", main_fn_name);
    }

    let main_fn_result = unsafe {
        // Try to find the -main function
        type MainFunc = unsafe extern "C" fn(*mut u8) -> *mut u8;
        engine.get_function::<MainFunc>(&main_fn_name).ok()
    };

    if let Some(main_fn) = main_fn_result {
        if debug {
            println!("   [DEBUG] Found -main function, calling with {} args", extra_args.len());
        }

        // Build a vector of string Values from extra_args
        unsafe {
            use clorus_runtime::vector::{clorus_vector_empty, clorus_vector_conj};
            use clorus_runtime::value::{Value, clorus_value_string};

            // Create empty vector
            let mut args_vec = clorus_vector_empty();

            // Add each argument as a string Value
            for arg in &extra_args {
                let c_str = std::ffi::CString::new(arg.as_str())
                    .map_err(|_| "Invalid argument string".to_string())?;
                let arg_val = clorus_value_string(c_str.as_ptr());

                // Conj to vector
                args_vec = clorus_vector_conj(args_vec, arg_val);
            }

            // Call -main with the args vector (cast to *mut u8)
            last_result_ptr = main_fn.call(args_vec as *mut u8);

            if debug {
                println!("   [DEBUG] -main returned successfully");
            }
        }
    } else if !extra_args.is_empty() {
        // Warn if args were provided but no -main found
        eprintln!("Warning: Command line arguments provided but no -main function found");
        eprintln!("         Define (defn -main [& args] ...) to accept arguments");
    }

    // Display the result
    if !last_result_ptr.is_null() {
        unsafe {
            use clorus_runtime::value::{Value, ValueTag, clorus_value_as_long, clorus_value_as_double, clorus_value_as_bool, clorus_value_as_cstring, clorus_free_cstring};
            let value = last_result_ptr as *mut Value;

            match (*value).header().tag() {
                ValueTag::Long => {
                    let num = clorus_value_as_long(value);
                    println!("=> {}", num);
                }
                ValueTag::Double => {
                    let num = clorus_value_as_double(value);
                    println!("=> {}", num);
                }
                ValueTag::String => {
                    let c_str = clorus_value_as_cstring(value);
                    if !c_str.is_null() {
                        let rust_str = std::ffi::CStr::from_ptr(c_str);
                        println!("=> \"{}\"", rust_str.to_string_lossy());
                        clorus_free_cstring(c_str);
                    } else {
                        println!("=> <null string>");
                    }
                }
                ValueTag::Bool => {
                    let b = clorus_value_as_bool(value);
                    println!("=> {}", if b { "true" } else { "false" });
                }
                ValueTag::Nil => {
                    println!("=> nil");
                }
                _ => {
                    println!("=> {:?}", *value);
                }
            }
        }
    }

    Ok(())
}

pub fn repl(main_thread: bool) -> Result<(), String> {
    // Create config with main_thread setting
    let config = clorus_repl::ReplConfig {
        main_thread,
        ..Default::default()
    };

    // Direct library call - no process spawning
    clorus_repl::run_with_config(config)
}

pub fn replx(args: &[String]) -> Result<(), String> {
    // Parse replx-specific arguments
    let mut config = clorus_replx::AdaptiveConfig::default();

    for arg in args {
        match arg.as_str() {
            "--no-adapt" => config.enabled = false,
            "--warn" => config.level = clorus_replx::AdaptationLevel::Warn,
            "--silent" => config.level = clorus_replx::AdaptationLevel::Silent,
            "--main-thread" => config.main_thread = true,
            "--no-gui-detach" => config.auto_detach_gui = false,
            other => {
                eprintln!("Unknown replx option: {}", other);
                eprintln!("Run 'clorus help' for usage");
                return Err(format!("Unknown option: {}", other));
            }
        }
    }

    // Run extended REPL
    clorus_replx::run_with_config(config)
}


/// Load clorus-runtime dynamic library to make Value* operations available to JIT
/// The library must stay loaded for the duration of execution
fn load_runtime_library() -> Result<libloading::Library, String> {
    use std::env;

    // Determine library file name based on platform
    #[cfg(target_os = "macos")]
    let lib_name = "libclorus_runtime.dylib";

    #[cfg(target_os = "linux")]
    let lib_name = "libclorus_runtime.so";

    #[cfg(target_os = "windows")]
    let lib_name = "clorus_runtime.dll";

    // Try to find the library in multiple locations
    let mut lib_path = None;

    // Prefer matching profile in development to avoid symbol/version skew.
    #[cfg(debug_assertions)]
    let local_candidates = [
        Path::new("target/debug/deps").join(lib_name),
        Path::new("target/debug").join(lib_name),
        Path::new("target/release/deps").join(lib_name),
        Path::new("target/release").join(lib_name),
    ];
    #[cfg(not(debug_assertions))]
    let local_candidates = [
        Path::new("target/release/deps").join(lib_name),
        Path::new("target/release").join(lib_name),
        Path::new("target/debug/deps").join(lib_name),
        Path::new("target/debug").join(lib_name),
    ];

    for candidate in local_candidates {
        if candidate.exists() {
            lib_path = Some(candidate);
            break;
        }
    }

    // 3. Try to find it relative to the clorus executable (for installed version)
    if lib_path.is_none() {
        if let Ok(exe_path) = env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                // Check ../lib/ directory (typical installation layout)
                let installed_path = exe_dir.parent()
                    .map(|p| p.join("lib").join(lib_name));
                if let Some(p) = installed_path {
                    if p.exists() {
                        lib_path = Some(p);
                    }
                }

                // Check same directory as executable
                if lib_path.is_none() {
                    let same_dir = exe_dir.join(lib_name);
                    if same_dir.exists() {
                        lib_path = Some(same_dir);
                    }
                }
            }
        }
    }

    // 4. Try workspace target directory (for development)
    if lib_path.is_none() {
        // Walk up to find workspace root
        let mut current = env::current_dir().ok();
        while let Some(dir) = current {
            #[cfg(debug_assertions)]
            let workspace_candidates = [
                dir.join("target/debug/deps").join(lib_name),
                dir.join("target/debug").join(lib_name),
                dir.join("target/release/deps").join(lib_name),
                dir.join("target/release").join(lib_name),
            ];
            #[cfg(not(debug_assertions))]
            let workspace_candidates = [
                dir.join("target/release/deps").join(lib_name),
                dir.join("target/release").join(lib_name),
                dir.join("target/debug/deps").join(lib_name),
                dir.join("target/debug").join(lib_name),
            ];

            for candidate in workspace_candidates {
                if candidate.exists() {
                    lib_path = Some(candidate);
                    break;
                }
            }
            if lib_path.is_some() {
                break;
            }
            current = dir.parent().map(|p| p.to_path_buf());
        }
    }

    let lib_path = lib_path.ok_or_else(|| {
        format!(
            "clorus-runtime library not found.
Searched:
  - target/release/deps/{}
  - target/release/{}
  - target/debug/deps/{}
  - target/debug/{}
  - Installed library directory
  - Workspace target directories

To fix: cargo build -p clorus-runtime --release",
            lib_name, lib_name, lib_name, lib_name
        )
    })?;

    load_dynamic_library(&lib_path)
}

/// Load clorus-std dynamic library to make fs functions available to JIT
/// The library must stay loaded for the duration of execution
fn load_std_library() -> Result<libloading::Library, String> {
    use std::env;

    // Determine library file name based on platform
    #[cfg(target_os = "macos")]
    let lib_name = "libclorus_std.dylib";

    #[cfg(target_os = "linux")]
    let lib_name = "libclorus_std.so";

    #[cfg(target_os = "windows")]
    let lib_name = "clorus_std.dll";

    // Try to find the library in target/{profile}
    let mut lib_path = None;

    #[cfg(debug_assertions)]
    let local_candidates = [Path::new("target/debug").join(lib_name), Path::new("target/release").join(lib_name)];
    #[cfg(not(debug_assertions))]
    let local_candidates = [Path::new("target/release").join(lib_name), Path::new("target/debug").join(lib_name)];

    for candidate in local_candidates {
        if candidate.exists() {
            lib_path = Some(candidate);
            break;
        }
    }

    // If not found in project dir, try workspace root
    if lib_path.is_none() {
        let current_dir = env::current_dir().map_err(|e| format!("Failed to get current dir: {}", e))?;

        // Go up directories to find workspace root
        let mut search_dir = current_dir.as_path();
        for _ in 0..5 {
            #[cfg(debug_assertions)]
            let search_candidates = [search_dir.join("target/debug").join(lib_name), search_dir.join("target/release").join(lib_name)];
            #[cfg(not(debug_assertions))]
            let search_candidates = [search_dir.join("target/release").join(lib_name), search_dir.join("target/debug").join(lib_name)];

            for candidate in search_candidates {
                if candidate.exists() {
                    lib_path = Some(candidate);
                    break;
                }
            }
            if lib_path.is_some() {
                break;
            }

            if let Some(parent) = search_dir.parent() {
                search_dir = parent;
            } else {
                break;
            }
        }
    }

    let lib_path = lib_path.ok_or_else(|| {
        format!(
            "clorus-std library not found. Run 'cargo build -p clorus-std --release' first.\n\
             Expected: target/release/{} or target/debug/{}",
            lib_name, lib_name
        )
    })?;

    // Load the library with RTLD_GLOBAL flag on Unix systems
    // This makes symbols available to LLVM JIT
    unsafe {
        #[cfg(unix)]
        {
            use libloading::os::unix::Library as UnixLibrary;
            use libloading::os::unix::RTLD_GLOBAL;
            use libloading::os::unix::RTLD_NOW;

            UnixLibrary::open(Some(&lib_path), RTLD_NOW | RTLD_GLOBAL)
                .map(|lib| lib.into())
                .map_err(|e| format!("Failed to load clorus-std library: {}", e))
        }

        #[cfg(not(unix))]
        {
            libloading::Library::new(&lib_path)
                .map_err(|e| format!("Failed to load clorus-std library: {}", e))
        }
    }
}

/// Load example-rust-lib dynamic library for FFI testing
/// The library must stay loaded for the duration of execution
fn load_example_library() -> Result<libloading::Library, String> {
    use std::env;

    // Determine library file name based on platform
    #[cfg(target_os = "macos")]
    let lib_name = "libexample_rust_lib.dylib";

    #[cfg(target_os = "linux")]
    let lib_name = "libexample_rust_lib.so";

    #[cfg(target_os = "windows")]
    let lib_name = "example_rust_lib.dll";

    // Try to find the library in target/{profile}
    let mut lib_path = None;

    #[cfg(debug_assertions)]
    let local_candidates = [Path::new("target/debug").join(lib_name), Path::new("target/release").join(lib_name)];
    #[cfg(not(debug_assertions))]
    let local_candidates = [Path::new("target/release").join(lib_name), Path::new("target/debug").join(lib_name)];

    for candidate in local_candidates {
        if candidate.exists() {
            lib_path = Some(candidate);
            break;
        }
    }

    // If not found in project dir, try workspace root
    if lib_path.is_none() {
        let current_dir = env::current_dir().map_err(|e| format!("Failed to get current dir: {}", e))?;

        // Go up directories to find workspace root
        let mut search_dir = current_dir.as_path();
        for _ in 0..5 {
            #[cfg(debug_assertions)]
            let search_candidates = [search_dir.join("target/debug").join(lib_name), search_dir.join("target/release").join(lib_name)];
            #[cfg(not(debug_assertions))]
            let search_candidates = [search_dir.join("target/release").join(lib_name), search_dir.join("target/debug").join(lib_name)];

            for candidate in search_candidates {
                if candidate.exists() {
                    lib_path = Some(candidate);
                    break;
                }
            }
            if lib_path.is_some() {
                break;
            }

            if let Some(parent) = search_dir.parent() {
                search_dir = parent;
            } else {
                break;
            }
        }
    }

    let lib_path = lib_path.ok_or_else(|| {
        format!(
            "example-rust-lib library not found. Run 'cargo build -p example-rust-lib --release' first.\n\
             Expected: target/release/{} or target/debug/{}",
            lib_name, lib_name
        )
    })?;

    load_dynamic_library(&lib_path)
}

/// Generic dynamic library loader with RTLD_GLOBAL flag
fn load_dynamic_library(lib_path: &Path) -> Result<libloading::Library, String> {
    // Load the library with RTLD_GLOBAL flag on Unix systems
    // This makes symbols available to LLVM JIT
    unsafe {
        #[cfg(unix)]
        {
            use libloading::os::unix::Library as UnixLibrary;
            use libloading::os::unix::RTLD_GLOBAL;
            use libloading::os::unix::RTLD_NOW;

            UnixLibrary::open(Some(lib_path), RTLD_NOW | RTLD_GLOBAL)
                .map(|lib| lib.into())
                .map_err(|e| format!("Failed to load library {}: {}", lib_path.display(), e))
        }

        #[cfg(not(unix))]
        {
            libloading::Library::new(lib_path)
                .map_err(|e| format!("Failed to load library {}: {}", lib_path.display(), e))
        }
    }
}

/// Load clorus-core dynamic library to make clorus.core functions available to JIT
fn load_core_library() -> Result<libloading::Library, String> {
    use std::env;

    // Determine library file name based on platform
    #[cfg(target_os = "macos")]
    let lib_name = "libclorus_core.dylib";

    #[cfg(target_os = "linux")]
    let lib_name = "libclorus_core.so";

    #[cfg(target_os = "windows")]
    let lib_name = "clorus_core.dll";

    // Try to find the library
    let mut lib_path = None;

    #[cfg(debug_assertions)]
    let local_candidates = [Path::new("target/debug").join(lib_name), Path::new("target/release").join(lib_name)];
    #[cfg(not(debug_assertions))]
    let local_candidates = [Path::new("target/release").join(lib_name), Path::new("target/debug").join(lib_name)];

    for candidate in local_candidates {
        if candidate.exists() {
            lib_path = Some(candidate);
            break;
        }
    }

    // If not found in current dir, try workspace root
    if lib_path.is_none() {
        let current_dir = env::current_dir().map_err(|e| format!("Failed to get current dir: {}", e))?;

        // Go up directories to find workspace root
        let mut search_dir = current_dir.as_path();
        for _ in 0..5 {
            #[cfg(debug_assertions)]
            let search_candidates = [search_dir.join("target/debug").join(lib_name), search_dir.join("target/release").join(lib_name)];
            #[cfg(not(debug_assertions))]
            let search_candidates = [search_dir.join("target/release").join(lib_name), search_dir.join("target/debug").join(lib_name)];

            for candidate in search_candidates {
                if candidate.exists() {
                    lib_path = Some(candidate);
                    break;
                }
            }
            if lib_path.is_some() {
                break;
            }

            if let Some(parent) = search_dir.parent() {
                search_dir = parent;
            } else {
                break;
            }
        }
    }

    let lib_path = lib_path.ok_or_else(|| {
        format!("clorus-core library not found")
    })?;

    load_dynamic_library(&lib_path)
}

// ============================================================================
// Workspace Commands
// ============================================================================

/// Build all members in a workspace
pub fn build_workspace(debug: bool) -> Result<(), String> {
    let (workspace_manifest, workspace_root) = Manifest::load_workspace()?;
    let workspace_config = workspace_manifest.workspace
        .ok_or("Not a workspace (no [workspace] section found)")?;

    let members = workspace_config.resolve_members(&workspace_root)?;

    println!("📦 Building workspace with {} member(s)", members.len());
    println!();

    let mut succeeded = Vec::new();
    let mut failed = Vec::new();

    for member_path in members {
        let member_name = member_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        println!("   Building {}...", member_name);

        // Change to member directory
        std::env::set_current_dir(&member_path)
            .map_err(|e| format!("Failed to cd to {}: {}", member_path.display(), e))?;

        // Build member
        match build_with_debug(debug) {
            Ok(_) => {
                succeeded.push(member_name.to_string());
            }
            Err(e) => {
                eprintln!("   ❌ Failed to build {}: {}", member_name, e);
                failed.push(member_name.to_string());
            }
        }

        // Return to workspace root
        std::env::set_current_dir(&workspace_root)
            .map_err(|e| format!("Failed to cd back to workspace root: {}", e))?;
    }

    println!();
    if failed.is_empty() {
        println!("✅ Workspace build complete - {} member(s) built", succeeded.len());
        Ok(())
    } else {
        println!("⚠️  Workspace build completed with errors:");
        println!("   ✓ Succeeded: {}", succeeded.join(", "));
        println!("   ❌ Failed: {}", failed.join(", "));
        Err(format!("{} member(s) failed to build", failed.len()))
    }
}

/// Clean all members in a workspace
pub fn clean_workspace() -> Result<(), String> {
    let (workspace_manifest, workspace_root) = Manifest::load_workspace()?;
    let workspace_config = workspace_manifest.workspace
        .ok_or("Not a workspace (no [workspace] section found)")?;

    let members = workspace_config.resolve_members(&workspace_root)?;

    println!("🧹 Cleaning workspace with {} member(s)", members.len());
    println!();

    for member_path in members {
        let member_name = member_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        println!("   Cleaning {}...", member_name);

        std::env::set_current_dir(&member_path)
            .map_err(|e| format!("Failed to cd to {}: {}", member_path.display(), e))?;

        // Clean member (ignore errors)
        let _ = clean();

        std::env::set_current_dir(&workspace_root)
            .map_err(|e| format!("Failed to cd back to workspace root: {}", e))?;
    }

    // Clean workspace-level target directory
    let workspace_target = workspace_root.join("target");
    if workspace_target.exists() {
        println!("   Cleaning workspace target/...");
        fs::remove_dir_all(&workspace_target)
            .map_err(|e| format!("Failed to clean workspace target: {}", e))?;
    }

    println!();
    println!("✅ Workspace cleaned");
    Ok(())
}

/// Package all members in a workspace to output directory
pub fn pack_workspace(output_dir: Option<String>) -> Result<(), String> {
    let (workspace_manifest, workspace_root) = Manifest::load_workspace()?;
    let workspace_config = workspace_manifest.workspace
        .ok_or("Not a workspace (no [workspace] section found)")?;

    let members = workspace_config.resolve_members(&workspace_root)?;

    // Determine output directory
    let output_dir = output_dir.unwrap_or_else(|| "dist".to_string());
    let output_path = workspace_root.join(&output_dir);
    fs::create_dir_all(&output_path)
        .map_err(|e| format!("Failed to create output dir: {}", e))?;

    println!("📦 Packaging workspace to {}", output_path.display());
    println!();

    let mut packaged = Vec::new();
    let mut skipped = Vec::new();

    for member_path in members {
        let member_name = member_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        println!("   Packaging {}...", member_name);

        std::env::set_current_dir(&member_path)
            .map_err(|e| format!("Failed to cd to {}: {}", member_path.display(), e))?;

        // Load member manifest to check if it's a library
        match Manifest::find_in_current_dir() {
            Ok(member_manifest) => {
                // Only package libraries (no entry point or explicit lib.clrs)
                let is_library = member_manifest.build.entry.is_none()
                    || member_manifest.build.entry.as_deref() == Some("src/lib.clrs")
                    || member_manifest.build.entry.as_deref() == Some("lib.clrs");

                if is_library {
                    let clip_name = format!("{}-{}.clip",
                        member_manifest.package.name,
                        member_manifest.package.version);

                    // Build as library first
                    if let Err(e) = build_lib() {
                        eprintln!("      ⚠️  Failed to build {}: {}", member_name, e);
                        std::env::set_current_dir(&workspace_root)
                            .map_err(|e| format!("Failed to cd back: {}", e))?;
                        continue;
                    }

                    // Pack member
                    match crate::pack::pack(Some(clip_name.clone())) {
                        Ok(_) => {
                            // Move .clip to workspace output directory
                            let clip_path = PathBuf::from(&clip_name);
                            if clip_path.exists() {
                                let dest = output_path.join(&clip_name);
                                fs::copy(&clip_path, &dest)
                                    .map_err(|e| format!("Failed to copy .clip: {}", e))?;
                                fs::remove_file(&clip_path)
                                    .map_err(|e| format!("Failed to remove .clip: {}", e))?;

                                packaged.push(clip_name);
                            }
                        }
                        Err(e) => {
                            eprintln!("      ⚠️  Failed to pack {}: {}", member_name, e);
                        }
                    }
                } else {
                    println!("      Skipped (not a library)");
                    skipped.push(member_name.to_string());
                }
            }
            Err(e) => {
                eprintln!("      ⚠️  Failed to load manifest: {}", e);
            }
        }

        std::env::set_current_dir(&workspace_root)
            .map_err(|e| format!("Failed to cd back to workspace root: {}", e))?;
    }

    println!();
    if !packaged.is_empty() {
        println!("✅ Packaged {} member(s):", packaged.len());
        for clip in &packaged {
            println!("   📦 {}", clip);
        }
    }

    if !skipped.is_empty() {
        println!();
        println!("ℹ️  Skipped {} member(s) (applications):", skipped.len());
        for name in &skipped {
            println!("   → {}", name);
        }
    }

    println!();
    println!("Output directory: {}", output_path.display());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{new, new_with_template, NewTemplate};
    use std::env;
    use std::fs;
    use tempfile::tempdir;

    struct CwdGuard(std::path::PathBuf);

    impl Drop for CwdGuard {
        fn drop(&mut self) {
            let _ = env::set_current_dir(&self.0);
        }
    }

    #[test]
    fn new_project_generates_modern_main_template() {
        let tmp = tempdir().expect("failed to create tempdir");
        let original_cwd = env::current_dir().expect("failed to get cwd");
        let _cwd_guard = CwdGuard(original_cwd);
        env::set_current_dir(tmp.path()).expect("failed to cd to tempdir");

        let project_name = "sample-app";
        new(project_name).expect("clorus new failed");

        let main_path = tmp.path().join(project_name).join("src/main.clrs");
        let main_content = fs::read_to_string(&main_path).expect("failed to read generated main.clrs");

        assert!(
            main_content.contains("(ns main)"),
            "generated template should include namespace declaration"
        );
        assert!(
            main_content.contains("(defn -main [& _args]"),
            "generated template should include -main entrypoint"
        );

    }

    #[test]
    fn new_project_rust_interop_template_includes_rust_dependency_and_ns_rust_clause() {
        let tmp = tempdir().expect("failed to create tempdir");
        let original_cwd = env::current_dir().expect("failed to get cwd");
        let _cwd_guard = CwdGuard(original_cwd);
        env::set_current_dir(tmp.path()).expect("failed to cd to tempdir");

        let project_name = "interop-app";
        new_with_template(project_name, NewTemplate::RustInterop).expect("clorus new failed");

        let manifest_path = tmp.path().join(project_name).join("Clorus.toml");
        let manifest = fs::read_to_string(&manifest_path).expect("failed to read manifest");
        assert!(
            manifest.contains("[rust-dependencies]"),
            "rust interop template should include rust-dependencies section"
        );
        assert!(
            manifest.contains("libm = { version = \"0.2\", interface = \"interfaces/libm.clri\" }"),
            "rust interop template should include libm version+interface dependency"
        );

        let main_path = tmp.path().join(project_name).join("src/main.clrs");
        let main_content = fs::read_to_string(&main_path).expect("failed to read generated main.clrs");
        assert!(
            main_content.contains("(:rust [libm :as m])"),
            "rust interop template should include :rust ns import"
        );
        assert!(
            !main_content.contains("(use rust.libm)"),
            "rust interop template should avoid redundant use rust import"
        );

        let interface_path = tmp.path().join(project_name).join("interfaces/libm.clri");
        let interface_content = fs::read_to_string(&interface_path).expect("failed to read generated libm.clri");
        assert!(
            interface_content.contains("(interface libm"),
            "rust interop template should generate libm.clri interface file"
        );
    }
}
