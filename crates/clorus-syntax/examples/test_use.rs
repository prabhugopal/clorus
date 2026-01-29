use clorus_syntax::parse;

fn main() {
    println!("Testing (use rust.fs) parsing...\n");

    // Test 1: Simple use
    let code1 = "(use rust.fs)";
    match parse(code1) {
        Ok(exprs) => println!("✅ Parsed: {:?}\n", exprs[0]),
        Err(e) => println!("❌ Error: {}\n", e),
    }

    // Test 2: Use with imports
    let code2 = "(use rust.fs [read write exists?])";
    match parse(code2) {
        Ok(exprs) => println!("✅ Parsed with imports: {:?}\n", exprs[0]),
        Err(e) => println!("❌ Error: {}\n", e),
    }

    // Test 3: Multiple use statements
    let code3 = r#"
        (use rust.fs)
        (use rust.path [exists? join])
        (def x 10)
    "#;
    match parse(code3) {
        Ok(exprs) => {
            println!("✅ Parsed multiple statements:");
            for (i, expr) in exprs.iter().enumerate() {
                println!("  {}. {:?}", i+1, expr);
            }
        }
        Err(e) => println!("❌ Error: {}\n", e),
    }
}
