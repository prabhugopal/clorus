/// Example: Complete rust.fs function call demonstration
///
/// Shows that fs/* functions can be called with string literals

use inkwell::context::Context;
use clorus_codegen::CodeGen;
use clorus_syntax::parse;

fn main() {
    println!("=== rust.fs Function Calls Demo ===\n");

    let code = r#"
        (use rust.fs)

        ; Test fs/write
        (def write-result (fs/write "test.txt" "Hello, Clorus!"))

        ; Test fs/exists?
        (def file-exists (fs/exists? "test.txt"))

        ; Test fs/read
        (def content (fs/read "test.txt"))

        ; Test fs/append
        (def append-result (fs/append "test.txt" "\nNew line"))

        ; Test fs/copy
        (def copy-result (fs/copy "test.txt" "test-copy.txt"))

        ; Test fs/is-file?
        (def is-file (fs/is-file? "test.txt"))

        ; Test fs/is-dir?
        (def is-dir (fs/is-dir? "."))

        ; Test fs/create-dir
        (def mkdir-result (fs/create-dir "new-dir"))

        ; Test fs/rename
        (def rename-result (fs/rename "test-copy.txt" "test-renamed.txt"))

        ; Test fs/remove
        (def remove-result (fs/remove "test-renamed.txt"))
    "#;

    println!("Clorus code:");
    println!("{}", code);
    println!("\n--- Parsing ---\n");

    let exprs = match parse(code) {
        Ok(e) => {
            println!("✅ Parsed {} expressions", e.len());
            e
        }
        Err(e) => {
            println!("❌ Parse error: {}", e);
            return;
        }
    };

    println!("\n--- Compiling to LLVM IR ---\n");

    let context = Context::create();
    let mut codegen = CodeGen::new(&context, "fs_calls_demo");

    let mut success_count = 0;
    let mut error_count = 0;

    for (i, expr) in exprs.iter().enumerate() {
        let fn_name = format!("expr_{}", i);
        match codegen.wrap_in_function(expr, &fn_name) {
            Ok(_) => {
                println!("✅ Compiled {}: {:?}", fn_name, expr);
                success_count += 1;
            }
            Err(e) => {
                println!("❌ Error in {}: {}", fn_name, e);
                error_count += 1;
            }
        }
    }

    println!("\n--- Summary ---\n");
    println!("✅ Success: {}", success_count);
    println!("❌ Errors: {}", error_count);

    if error_count == 0 {
        println!("\n--- Generated LLVM IR (excerpt) ---\n");
        let ir = codegen.get_module().print_to_string().to_string();

        // Show first 100 lines of IR
        let lines: Vec<&str> = ir.lines().collect();
        for (i, line) in lines.iter().take(100).enumerate() {
            println!("{:4} | {}", i + 1, line);
        }

        if lines.len() > 100 {
            println!("... ({} more lines)", lines.len() - 100);
        }

        println!("\n✅ All fs/* functions compiled successfully!");
        println!("\nYou can see:");
        println!("  - Function declarations for all fs functions");
        println!("  - Global string constants for file paths");
        println!("  - Function calls to clorus_fs_* functions");
        println!("  - Type conversions (i32->f64, ptr->i64->f64)");
    }
}
