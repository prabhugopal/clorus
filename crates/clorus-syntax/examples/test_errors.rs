use clorus_syntax::parse;

fn main() {
    // Test 1: Mismatched brackets [x)
    println!("=== Test 1: test-error.clrs ===");
    let code1 = "(defn bad [x) x)";
    match parse(code1) {
        Ok(_) => println!("OK: Parsed successfully (unexpected!)"),
        Err(e) => println!("ERROR: {}", e),
    }

    println!("\n=== Test 2: test-bad.clrs ===");
    // Test 2: Incomplete function
    let code2 = "(defn test [x y";
    match parse(code2) {
        Ok(_) => println!("OK: Parsed successfully (unexpected!)"),
        Err(e) => println!("ERROR: {}", e),
    }
}
