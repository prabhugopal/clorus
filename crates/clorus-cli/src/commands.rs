/// Commands for the Clorus CLI tool
use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashSet;
use std::io::{Write, Read};
use crate::manifest::Manifest;
use clorus_syntax::Expr;

pub fn new(name: &str) -> Result<(), String> {
    let project_path = Path::new(name);

    if project_path.exists() {
        return Err(format!("Directory '{}' already exists", name));
    }

    // Create project structure
    fs::create_dir_all(project_path.join("src"))
        .map_err(|e| format!("Failed to create directories: {}", e))?;

    // Create Clorus.toml
    let manifest_content = format!(
        r#"[package]
name = "{}"
version = "0.1.0"
authors = []

[build]
entry = "src/main.clrs"
"#,
        name
    );

    fs::write(project_path.join("Clorus.toml"), manifest_content)
        .map_err(|e| format!("Failed to create Clorus.toml: {}", e))?;

    // Create src/main.clrs
    let main_content = r#"; Clorus main file
(def main-result
  (let [x 10
        y 20]
    (+ x y)))

; Uncomment to define a function
; (defn greet [name]
;   (println "Hello" name))

main-result
"#;

    fs::write(project_path.join("src/main.clrs"), main_content)
        .map_err(|e| format!("Failed to create main.clrs: {}", e))?;

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

    println!("     → Found: {}", file_path.display());

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
                let dep_exprs = load_module_recursive(&spec.module, loaded_modules, base_path, clip_namespaces)?;
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
                let dep_exprs = load_module_recursive(&spec.module, loaded_modules, base_path, clip_namespaces)?;
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
fn namespace_to_path(namespace: &str, base_path: &Path) -> Result<PathBuf, String> {
    let parts: Vec<&str> = namespace.split('.').collect();

    if parts.is_empty() {
        return Err(format!("Invalid namespace: {}", namespace));
    }

    // Build path: src/coral/widgets.clrs
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

    // If no entry specified, this is a library - nothing to check
    let entry = match &manifest.build.entry {
        Some(e) => e,
        None => {
            println!("   Skipping check for library package {} v{}",
                manifest.package.name, manifest.package.version);
            return Ok(());
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

    let target_dir = Path::new("target");

    if target_dir.exists() {
        fs::remove_dir_all(target_dir)
            .map_err(|e| format!("Failed to remove target directory: {}", e))?;
        println!("      Removed target/");
    } else {
        println!("      Nothing to clean (target/ doesn't exist)");
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

fn build_internal(lib_mode: bool, debug: bool) -> Result<(), String> {
    let manifest = Manifest::find_in_current_dir()?;

    // If no entry specified, this is a library - skip building executable
    let entry = match &manifest.build.entry {
        Some(e) => e.clone(),
        None => {
            println!("   Skipping build for library package {} v{}",
                manifest.package.name, manifest.package.version);
            println!("   (No entry point specified - this is a library)");
            return Ok(());
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

    // Load and parse stdlib/core.clr first (provides inc, dec, range, for, doseq, etc.)
    let stdlib_path = Path::new("stdlib/core.clr");
    let mut all_exprs = Vec::new();

    if stdlib_path.exists() {
        let stdlib_source = fs::read_to_string(stdlib_path)
            .map_err(|e| format!("Failed to read stdlib/core.clr: {}", e))?;

        let stdlib_exprs = clorus::parse_and_expand(&stdlib_source)
            .map_err(|e| format!("Parse error in stdlib/core.clr: {}", e))?;

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
            let alt_stdlib = compiler_dir.join("stdlib/core.clr");
            if alt_stdlib.exists() {
                let stdlib_source = fs::read_to_string(&alt_stdlib)
                    .map_err(|e| format!("Failed to read stdlib/core.clr: {}", e))?;

                let stdlib_exprs = clorus::parse_and_expand(&stdlib_source)
                    .map_err(|e| format!("Parse error in stdlib/core.clr: {}", e))?;

                all_exprs.extend(stdlib_exprs);
            }
        }
    }

    // Load and parse stdlib/transducers.clr (provides transducer support)
    let transducers_path = Path::new("stdlib/transducers.clr");
    if transducers_path.exists() {
        let transducers_source = fs::read_to_string(transducers_path)
            .map_err(|e| format!("Failed to read stdlib/transducers.clr: {}", e))?;

        let transducers_exprs = clorus::parse_and_expand(&transducers_source)
            .map_err(|e| format!("Parse error in stdlib/transducers.clr: {}", e))?;

        if debug {
            println!("   [DEBUG] Loaded {} expressions from stdlib/transducers.clr", transducers_exprs.len());
        }
        all_exprs.extend(transducers_exprs);
    } else {
        // Try relative to compiler location
        let compiler_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().and_then(|p| p.parent()).map(|p| p.to_path_buf()));

        if let Some(compiler_dir) = compiler_dir {
            let alt_transducers = compiler_dir.join("stdlib/transducers.clr");
            if alt_transducers.exists() {
                let transducers_source = fs::read_to_string(&alt_transducers)
                    .map_err(|e| format!("Failed to read stdlib/transducers.clr: {}", e))?;

                let transducers_exprs = clorus::parse_and_expand(&transducers_source)
                    .map_err(|e| format!("Parse error in stdlib/transducers.clr: {}", e))?;

                all_exprs.extend(transducers_exprs);
            }
        }
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

    for expr in &entry_exprs {
        // Check top-level Expr::Require
        if let Expr::Require { specs } = expr {
            for spec in specs {
                // Check if this module is from a .clip package
                let is_clip = clip_namespace_prefixes.iter().any(|prefix| {
                    spec.module.starts_with(prefix)
                });

                if is_clip {
                    println!("   Skipping module (from .clip): {}", spec.module);
                    continue;
                }

                println!("   Loading module: {}", spec.module);
                let dep_exprs = load_module_recursive(&spec.module, &mut loaded_modules, &base_path, &clip_namespace_prefixes)?;
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
                    println!("   Skipping module (from .clip): {}", spec.module);
                    continue;
                }

                println!("   Loading module: {}", spec.module);
                let dep_exprs = load_module_recursive(&spec.module, &mut loaded_modules, &base_path, &clip_namespace_prefixes)?;
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

    // Find runtime library - search in current dir and workspace root
    let mut runtime_lib: Option<String> = None;

    // Try current directory first (including deps directories where Cargo places libraries)
    for path in &[
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
    ] {
        if Path::new(path).exists() {
            runtime_lib = Some(path.to_string());
            break;
        }
    }

    // Try to find relative to clorus executable (for installed version or running from workspace)
    if runtime_lib.is_none() {
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                // Try relative to executable: deps/libclorus_runtime.a (same directory as exe)
                let same_dir_deps = exe_dir.join("deps/libclorus_runtime.a");
                if same_dir_deps.exists() {
                    runtime_lib = Some(same_dir_deps.to_string_lossy().to_string());
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
        Searched for libclorus_runtime.a in:\n\
        - target/release/\n\
        - target/debug/\n\
        - ../target/release/\n\
        - ../target/debug/\n\
        - ../../target/release/\n\
        - ../../target/debug/\n\
        \n\
        To fix this:\n\
        1. Build the runtime: cargo build -p clorus-runtime --release\n\
        2. Or copy libclorus_runtime.a to your project's target directory".to_string()
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
    use clorus_codegen::namespace_context::NamespaceContext;

    for module_name in modules {
        if loaded.contains(module_name) {
            continue; // Already loaded
        }

        // Convert module name to file path: math -> math.clrs/math.clr, text-field.core -> text-field/core.clrs
        // We preserve hyphens in the file path (e.g., text-field.core -> text-field/core.clrs)
        let module_path = module_name.replace('.', "/");

        // Try each source directory in order, checking both .clrs and .clr extensions
        let mut module_file = None;
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
                }

                let mut ns_ctx = NamespaceContext::default_namespace();
                ns_ctx.current = name.clone();

                // Process requires
                for req_spec in requires {
                    if let Some(ref alias) = req_spec.alias {
                        ns_ctx.aliases.insert(alias.clone(), req_spec.module.clone());
                    }
                    for symbol in &req_spec.refer {
                        ns_ctx.imports.insert(symbol.clone(), req_spec.module.clone());
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
        // Use JIT mode (faster but doesn't work with .clip dependencies)
        if debug {
            println!("   Mode: JIT compilation (--jit flag)");
            println!();
        }
        run_jit_internal(debug, extra_args)
    } else {
        // Default: compile + run (like cargo run)
        // Works with .clip dependencies and all features
        if debug {
            println!("   Mode: Compile and run (like cargo run)");
            println!("   Tip: Use --jit for faster iteration on small projects");
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

/// JIT compilation mode - fast but limited
/// Doesn't work with .clip dependencies due to LLVM bitcode compatibility
fn run_jit_internal(debug: bool, extra_args: Vec<String>) -> Result<(), String> {
    let manifest = Manifest::find_in_current_dir()?;

    // If no entry specified, this is a library - cannot run
    let entry = match &manifest.build.entry {
        Some(e) => e.clone(),
        None => {
            return Err("Cannot run library package (no entry point specified)".to_string());
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

    // Load and parse stdlib/core.clr first (provides inc, dec, range, for, doseq, etc.)
    let stdlib_path = Path::new("stdlib/core.clr");
    let mut all_exprs = Vec::new();

    if stdlib_path.exists() {
        let stdlib_source = fs::read_to_string(stdlib_path)
            .map_err(|e| format!("Failed to read stdlib/core.clr: {}", e))?;

        let stdlib_exprs = clorus::parse_and_expand(&stdlib_source)
            .map_err(|e| format!("Parse error in stdlib/core.clr: {}", e))?;

        if debug {
            println!("   [DEBUG] Loaded {} expressions from stdlib", stdlib_exprs.len());
        }
        all_exprs.extend(stdlib_exprs);
    } else if debug {
        println!("   [DEBUG] stdlib/core.clr not found, stdlib functions unavailable");
    }

    // Load and parse stdlib/transducers.clr (provides transducer support)
    let transducers_path = Path::new("stdlib/transducers.clr");
    if transducers_path.exists() {
        let transducers_source = fs::read_to_string(transducers_path)
            .map_err(|e| format!("Failed to read stdlib/transducers.clr: {}", e))?;

        let transducers_exprs = clorus::parse_and_expand(&transducers_source)
            .map_err(|e| format!("Parse error in stdlib/transducers.clr: {}", e))?;

        if debug {
            println!("   [DEBUG] Loaded {} expressions from stdlib/transducers.clr", transducers_exprs.len());
        }
        all_exprs.extend(transducers_exprs);
    } else if debug {
        println!("   [DEBUG] stdlib/transducers.clr not found, transducers unavailable");
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
    let mut stdlib_expr_count = if stdlib_path.exists() {
        if let Ok(stdlib_source) = fs::read_to_string(stdlib_path) {
            if let Ok(stdlib_exprs) = clorus::parse_and_expand(&stdlib_source) {
                stdlib_exprs.len()
            } else {
                0
            }
        } else {
            0
        }
    } else {
        0
    };

    // Add transducers expressions to count
    stdlib_expr_count += if transducers_path.exists() {
        if let Ok(transducers_source) = fs::read_to_string(transducers_path) {
            if let Ok(transducers_exprs) = clorus::parse_and_expand(&transducers_source) {
                transducers_exprs.len()
            } else {
                0
            }
        } else {
            0
        }
    } else {
        0
    };

    if debug {
        println!("   [DEBUG] Compiling {} stdlib expressions before modules", stdlib_expr_count);
    }

    // Compile stdlib expressions into CodeGen so modules can use them
    // Use "user" namespace for stdlib (no mangling)
    use clorus_codegen::namespace_context::NamespaceContext;
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
            }

            let mut ns_ctx = NamespaceContext::default_namespace();
            ns_ctx.current = name.clone();

            // Process requires
            for req_spec in requires {
                if let Some(ref alias) = req_spec.alias {
                    ns_ctx.aliases.insert(alias.clone(), req_spec.module.clone());
                }
                for symbol in &req_spec.refer {
                    ns_ctx.imports.insert(symbol.clone(), req_spec.module.clone());
                }
            }

            // Process rust imports
            for rust_import in rust_imports {
                if let Some(ref alias) = rust_import.alias {
                    let rust_module = format!("rust.{}", rust_import.library.replace('-', "_"));
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

    // 1. Try current project's target/release
    let release_path = Path::new("target/release").join(lib_name);
    if release_path.exists() {
        lib_path = Some(release_path);
    }

    // 2. Try current project's target/debug
    if lib_path.is_none() {
        let debug_path = Path::new("target/debug").join(lib_name);
        if debug_path.exists() {
            lib_path = Some(debug_path);
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
            let workspace_release = dir.join("target/release").join(lib_name);
            if workspace_release.exists() {
                lib_path = Some(workspace_release);
                break;
            }
            let workspace_debug = dir.join("target/debug").join(lib_name);
            if workspace_debug.exists() {
                lib_path = Some(workspace_debug);
                break;
            }
            current = dir.parent().map(|p| p.to_path_buf());
        }
    }

    let lib_path = lib_path.ok_or_else(|| {
        format!(
            "clorus-runtime library not found.
Searched:
  - target/release/{}
  - target/debug/{}
  - Installed library directory
  - Workspace target directories

To fix: cargo build -p clorus-runtime --release",
            lib_name, lib_name
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

    // Try to find the library in target/release or target/debug
    let mut lib_path = None;

    // First try release build
    let release_path = Path::new("target/release").join(lib_name);
    if release_path.exists() {
        lib_path = Some(release_path);
    } else {
        // Try debug build
        let debug_path = Path::new("target/debug").join(lib_name);
        if debug_path.exists() {
            lib_path = Some(debug_path);
        }
    }

    // If not found in project dir, try workspace root
    if lib_path.is_none() {
        let current_dir = env::current_dir().map_err(|e| format!("Failed to get current dir: {}", e))?;

        // Go up directories to find workspace root
        let mut search_dir = current_dir.as_path();
        for _ in 0..5 {
            let release_path = search_dir.join("target/release").join(lib_name);
            if release_path.exists() {
                lib_path = Some(release_path);
                break;
            }

            let debug_path = search_dir.join("target/debug").join(lib_name);
            if debug_path.exists() {
                lib_path = Some(debug_path);
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

    // Try to find the library in target/release or target/debug
    let mut lib_path = None;

    // First try release build
    let release_path = Path::new("target/release").join(lib_name);
    if release_path.exists() {
        lib_path = Some(release_path);
    } else {
        // Try debug build
        let debug_path = Path::new("target/debug").join(lib_name);
        if debug_path.exists() {
            lib_path = Some(debug_path);
        }
    }

    // If not found in project dir, try workspace root
    if lib_path.is_none() {
        let current_dir = env::current_dir().map_err(|e| format!("Failed to get current dir: {}", e))?;

        // Go up directories to find workspace root
        let mut search_dir = current_dir.as_path();
        for _ in 0..5 {
            let release_path = search_dir.join("target/release").join(lib_name);
            if release_path.exists() {
                lib_path = Some(release_path);
                break;
            }

            let debug_path = search_dir.join("target/debug").join(lib_name);
            if debug_path.exists() {
                lib_path = Some(debug_path);
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

    // Try release build first
    let release_path = Path::new("target/release").join(lib_name);
    if release_path.exists() {
        lib_path = Some(release_path);
    } else {
        // Try debug build
        let debug_path = Path::new("target/debug").join(lib_name);
        if debug_path.exists() {
            lib_path = Some(debug_path);
        }
    }

    // If not found in current dir, try workspace root
    if lib_path.is_none() {
        let current_dir = env::current_dir().map_err(|e| format!("Failed to get current dir: {}", e))?;

        // Go up directories to find workspace root
        let mut search_dir = current_dir.as_path();
        for _ in 0..5 {
            let release_path = search_dir.join("target/release").join(lib_name);
            if release_path.exists() {
                lib_path = Some(release_path);
                break;
            }

            let debug_path = search_dir.join("target/debug").join(lib_name);
            if debug_path.exists() {
                lib_path = Some(debug_path);
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
