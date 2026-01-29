/// Commands for the Clorus CLI tool
use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashSet;
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

pub fn check() -> Result<(), String> {
    let manifest = Manifest::find_in_current_dir()?;
    let entry_path = Path::new(&manifest.build.entry);

    if !entry_path.exists() {
        return Err(format!("Entry file '{}' not found", manifest.build.entry));
    }

    println!("   Checking {} v{}", manifest.package.name, manifest.package.version);

    let source = fs::read_to_string(entry_path)
        .map_err(|e| format!("Failed to read {}: {}", manifest.build.entry, e))?;

    // Parse to check syntax (with macro expansion)
    clorus::parse_and_expand(&source)
        .map_err(|e| format!("Parse error in {}: {}", manifest.build.entry, e))?;

    println!("    Finished checking {} in 0.00s", manifest.package.name);

    Ok(())
}

pub fn build() -> Result<(), String> {
    let manifest = Manifest::find_in_current_dir()?;
    let entry_path = Path::new(&manifest.build.entry);

    if !entry_path.exists() {
        return Err(format!("Entry file '{}' not found", manifest.build.entry));
    }

    // Process Rust dependencies (auto-generate FFI and compile)
    let rust_ffi = crate::rust_ffi::RustFfiProcessor::process_dependencies(&manifest, true)?;

    println!("   Compiling {} v{}", manifest.package.name, manifest.package.version);

    let source = fs::read_to_string(entry_path)
        .map_err(|e| format!("Failed to read {}: {}", manifest.build.entry, e))?;

    // Parse and expand macros
    let exprs = clorus::parse_and_expand(&source)
        .map_err(|e| format!("Parse error: {}", e))?;

    if exprs.is_empty() {
        return Err("No expressions to compile".to_string());
    }

    // Compile to LLVM IR and create executable
    use inkwell::context::Context;
    use inkwell::targets::{Target, InitializationConfig, TargetMachine, RelocMode, CodeModel, FileType};
    use inkwell::OptimizationLevel;
    use clorus::CodeGen;

    let context = Context::create();
    let mut codegen = CodeGen::new(&context, &manifest.package.name);

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
    for (i, expr) in exprs.iter().enumerate() {
        let fn_name = format!("expr_{}", i);
        codegen.wrap_in_function(expr, &fn_name)
            .map_err(|e| format!("Compile error: {}", e))?;
        function_names.push(fn_name);
    }

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

    // Call each compiled expression and print the last one
    let mut last_result: Option<inkwell::values::PointerValue> = None;
    for (i, fn_name) in function_names.iter().enumerate() {
        // Get the function from the module
        if let Some(func) = codegen.get_module().get_function(fn_name) {
            let result = builder.build_call(func, &[], "call").unwrap();
            let result_ptr = result.try_as_basic_value().left().unwrap().into_pointer_value();

            // If this is the last expression, print it
            if i == function_names.len() - 1 {
                last_result = Some(result_ptr);
            }
        }
    }

    // Print the final result if we have one
    if let Some(result_ptr) = last_result {
        // clorus_value_as_number is already declared by the runtime
        // Just get it from the module
        let value_as_number_fn = codegen.get_module()
            .get_function("clorus_value_as_number")
            .expect("clorus_value_as_number should be declared by runtime");

        let num_result = builder.build_call(
            value_as_number_fn,
            &[result_ptr.into()],
            "extract_num"
        ).unwrap();
        let num_val = num_result.try_as_basic_value().left().unwrap().into_float_value();

        // Print the number
        builder.build_call(
            printf_fn,
            &[float_format.as_pointer_value().into(), num_val.into()],
            "printf_call"
        ).unwrap();
    }

    // Return 0 (success)
    let zero = context.i32_type().const_int(0, false);
    builder.build_return(Some(&zero)).unwrap();

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

    // Link the object file into an executable
    let exe_path = target_dir.join(&manifest.package.name);

    // Find runtime library - search in current dir and workspace root
    let mut runtime_lib: Option<String> = None;

    // Try current directory first
    for path in &[
        "target/release/libclorus_runtime.a",
        "target/debug/libclorus_runtime.a",
        "../target/release/libclorus_runtime.a",
        "../target/debug/libclorus_runtime.a",
        "../../target/release/libclorus_runtime.a",
        "../../target/debug/libclorus_runtime.a",
    ] {
        if Path::new(path).exists() {
            runtime_lib = Some(path.to_string());
            break;
        }
    }

    // Try to find in the clorus installation directory
    if runtime_lib.is_none() {
        // Get the directory where clorus binary is located
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(parent) = exe_path.parent() {
                // Try ../lib relative to bin directory
                let lib_path = parent.parent().and_then(|p| Some(p.join("lib/libclorus_runtime.a")));
                if let Some(path) = lib_path {
                    if path.exists() {
                        runtime_lib = Some(path.to_string_lossy().to_string());
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
) -> Result<(), String> {
    use clorus_codegen::namespace_context::NamespaceContext;

    for module_name in modules {
        if loaded.contains(module_name) {
            continue; // Already loaded
        }

        // Convert module name to file path: math -> src/math.clrs
        let module_path = module_name.replace('.', "/");
        let module_file = project_root.join("src").join(format!("{}.clrs", module_path));

        if !module_file.exists() {
            return Err(format!("Module file not found: {}", module_file.display()));
        }

        // Read and parse the module
        let source = fs::read_to_string(&module_file)
            .map_err(|e| format!("Failed to read {}: {}", module_file.display(), e))?;

        let exprs = clorus::parse_and_expand(&source)
            .map_err(|e| format!("Parse error in {}: {}", module_file.display(), e))?;

        // Extract and load transitive dependencies first
        let sub_modules = extract_required_modules(&exprs);
        if !sub_modules.is_empty() {
            load_and_compile_modules(&sub_modules, codegen, loaded, project_root)?;
        }

        // Compile this module's expressions
        for expr in &exprs {
            // Set namespace context if this is an ns declaration
            if let Expr::Ns { name, requires, rust_imports } = expr {
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


pub fn run(debug: bool) -> Result<(), String> {
    let manifest = Manifest::find_in_current_dir()?;
    let entry_path = Path::new(&manifest.build.entry);

    if !entry_path.exists() {
        return Err(format!("Entry file '{}' not found", manifest.build.entry));
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

    println!("   Compiling {} v{}", manifest.package.name, manifest.package.version);

    let source = fs::read_to_string(entry_path)
        .map_err(|e| format!("Failed to read {}: {}", manifest.build.entry, e))?;

    // Parse and expand macros
    let exprs = clorus::parse_and_expand(&source)
        .map_err(|e| format!("Parse error: {}", e))?;

    if exprs.is_empty() {
        return Err("No expressions to execute".to_string());
    }

    println!("    Finished dev [unoptimized] target(s) in 0.00s");
    println!("     Running `{}`", manifest.build.entry);
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
    use clorus_runtime::value::{clorus_value_number, clorus_release};

    // Ensure runtime symbols are linked by touching them
    unsafe {
        let _dummy = clorus_value_number(0.0);
        clorus_release(_dummy);
    }

    let context = Context::create();
    let mut codegen = CodeGen::new(&context, &manifest.package.name);

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

    // Load and compile required modules first
    let project_root = std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;

    let required_modules = extract_required_modules(&exprs);
    let mut loaded_modules = HashSet::new();

    if !required_modules.is_empty() {
        load_and_compile_modules(&required_modules, &mut codegen, &mut loaded_modules, &project_root)?;
    }

    // Update namespace context for entry file
    use clorus_codegen::namespace_context::NamespaceContext;
    for expr in &exprs {
        if let Expr::Ns { name, requires, rust_imports } = expr {
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
    for (i, expr) in exprs.iter().enumerate() {
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

    // Display the result
    if !last_result_ptr.is_null() {
        unsafe {
            use clorus_runtime::value::{Value, ValueTag, clorus_value_as_number, clorus_value_as_cstring, clorus_free_cstring};
            let value = last_result_ptr as *mut Value;

            match (*value).header().tag() {
                ValueTag::Number => {
                    let num = clorus_value_as_number(value);
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
                    let num = clorus_value_as_number(value);
                    println!("=> {}", if num != 0.0 { "true" } else { "false" });
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

pub fn repl() -> Result<(), String> {
    use std::process::Command;
    use std::env;

    // Check if we're in a project directory
    if let Ok(manifest) = Manifest::find_in_current_dir() {
        println!("Starting REPL for project: {} v{}", manifest.package.name, manifest.package.version);
        println!();
    }

    // Find the REPL binary - it should be next to the clorus binary
    let clorus_exe = env::current_exe()
        .map_err(|e| format!("Could not determine clorus executable path: {}", e))?;

    let clorus_dir = clorus_exe.parent()
        .ok_or("Could not determine clorus directory")?;

    let repl_exe = clorus_dir.join("repl");

    // Try to launch the REPL binary directly
    let status = if repl_exe.exists() {
        Command::new(&repl_exe)
            .status()
    } else {
        // Fallback: try PATH
        Command::new("repl")
            .status()
    };

    match status {
        Ok(exit_status) => {
            if exit_status.success() {
                Ok(())
            } else {
                Err("REPL exited with error".to_string())
            }
        }
        Err(e) => {
            eprintln!("Failed to launch REPL: {}", e);
            eprintln!();
            eprintln!("Note: Make sure 'repl' binary is built and in the same directory as 'clorus'");
            eprintln!("Build with: cargo build --release --bin repl");
            Err(format!("Could not launch REPL: {}", e))
        }
    }
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
