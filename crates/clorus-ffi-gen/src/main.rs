/// CLI tool for generating FFI bindings
use clorus_ffi_gen::FfiGenerator;
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: clorus-ffi-gen <rust-source-file> [options]");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  --save-wrappers <file>      Save C wrappers to file");
        eprintln!("  --save-llvm <file>          Save LLVM declarations to file");
        eprintln!("  --generate-module <file>    Generate complete module with wrappers");
        eprintln!();
        eprintln!("Example:");
        eprintln!("  clorus-ffi-gen src/lib.rs");
        eprintln!("  clorus-ffi-gen src/lib.rs --save-wrappers ffi_wrappers.rs");
        eprintln!("  clorus-ffi-gen src/lib.rs --save-llvm llvm_decl.rs");
        std::process::exit(1);
    }

    let source_file = &args[1];
    let path = Path::new(source_file);

    if !path.exists() {
        eprintln!("Error: File not found: {}", source_file);
        std::process::exit(1);
    }

    // Parse command-line options
    let mut save_wrappers: Option<String> = None;
    let mut save_llvm: Option<String> = None;
    let mut generate_module: Option<String> = None;

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--save-wrappers" => {
                if i + 1 < args.len() {
                    save_wrappers = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: --save-wrappers requires a filename");
                    std::process::exit(1);
                }
            }
            "--save-llvm" => {
                if i + 1 < args.len() {
                    save_llvm = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: --save-llvm requires a filename");
                    std::process::exit(1);
                }
            }
            "--generate-module" => {
                if i + 1 < args.len() {
                    generate_module = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: --generate-module requires a filename");
                    std::process::exit(1);
                }
            }
            _ => {
                eprintln!("Error: Unknown option: {}", args[i]);
                std::process::exit(1);
            }
        }
    }

    println!("🔍 Analyzing Rust source: {}", source_file);
    println!();

    let mut generator = FfiGenerator::new();

    match generator.parse_file(path) {
        Ok(()) => {
            println!("✅ Found {} public functions:", generator.functions.len());
            println!();

            for func in &generator.functions {
                println!("  • {}", func.name);
                for param in &func.params {
                    println!("      - {}: {}", param.name, param.type_name);
                }
                println!("      → {}", func.return_type);
                println!();
            }

            // Save files if requested
            if let Some(ref wrapper_file) = save_wrappers {
                match generator.save_c_wrappers(Path::new(wrapper_file)) {
                    Ok(()) => println!("✅ Saved C wrappers to: {}", wrapper_file),
                    Err(e) => {
                        eprintln!("❌ Error saving wrappers: {}", e);
                        std::process::exit(1);
                    }
                }
            }

            if let Some(ref llvm_file) = save_llvm {
                match generator.save_llvm_declarations(Path::new(llvm_file)) {
                    Ok(()) => println!("✅ Saved LLVM declarations to: {}", llvm_file),
                    Err(e) => {
                        eprintln!("❌ Error saving LLVM declarations: {}", e);
                        std::process::exit(1);
                    }
                }
            }

            if let Some(ref module_file) = generate_module {
                match generator.generate_ffi_module(path) {
                    Ok(module_content) => {
                        match std::fs::write(module_file, module_content) {
                            Ok(()) => println!("✅ Generated complete module: {}", module_file),
                            Err(e) => {
                                eprintln!("❌ Error writing module: {}", e);
                                std::process::exit(1);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("❌ Error generating module: {}", e);
                        std::process::exit(1);
                    }
                }
            }

            // Print to stdout if no save options specified
            if save_wrappers.is_none() && save_llvm.is_none() && generate_module.is_none() {
                println!("📝 Generating C wrappers...");
                println!();
                let wrappers = generator.generate_c_wrappers();
                println!("{}", wrappers);

                println!();
                println!("🔗 LLVM Declarations:");
                println!();
                let llvm = generator.generate_llvm_declarations();
                println!("{}", llvm);
            }

            println!("✅ FFI generation complete!");
        }
        Err(e) => {
            eprintln!("❌ Error: {}", e);
            std::process::exit(1);
        }
    }
}
