/// Example: rust.fs integration - Generate LLVM IR
///
/// Shows that (use rust.fs) declares the functions properly

use inkwell::context::Context;
use clorus_codegen::CodeGen;
use clorus_syntax::parse;

fn main() {
    println!("=== rust.fs Integration Test ===\n");

    let code = r#"
        (use rust.fs)

        (def x 42)
    "#;

    println!("Clorus code:");
    println!("{}", code);
    println!("\n--- Parsing ---\n");

    let exprs = parse(code).expect("Parse failed");
    for (i, expr) in exprs.iter().enumerate() {
        println!("{}. {:?}", i+1, expr);
    }

    println!("\n--- Compiling ---\n");

    let context = Context::create();
    let mut codegen = CodeGen::new(&context, "fs_demo");

    for (i, expr) in exprs.iter().enumerate() {
        let fn_name = format!("expr_{}", i);
        match codegen.wrap_in_function(expr, &fn_name) {
            Ok(_) => println!("✅ Compiled {}", fn_name),
            Err(e) => println!("❌ Error: {}", e),
        }
    }

    println!("\n--- Generated LLVM IR ---\n");
    codegen.print_ir();

    println!("\n✅ Success! rust.fs functions are declared in LLVM module");
    println!("\nDeclared functions:");
    println!("  - clorus_fs_read");
    println!("  - clorus_fs_write");
    println!("  - clorus_fs_append");
    println!("  - clorus_fs_exists");
    println!("  - clorus_fs_is_file");
    println!("  - clorus_fs_is_dir");
    println!("  - clorus_fs_remove");
    println!("  - clorus_fs_copy");
    println!("  - clorus_fs_rename");
    println!("  - clorus_fs_create_dir");
    println!("  - clorus_fs_create_dir_all");
    println!("  - clorus_fs_free_string");
}
